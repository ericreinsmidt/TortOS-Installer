// The installer's screens. Main: the cards in this computer (list_cards, every
// two seconds, so a card put in or taken out shows by itself), which device
// and what to do. Then a confirm before erasing, the write with its progress
// ("progress" and "finished" events from the app), and how it ended.

const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : null;
const $ = (id) => document.getElementById(id);
const DEVICE = { brick: "the Brick", pixel2: "the Pixel 2" };

const state = { cards: [], cardId: null, device: null, action: null, userDevice: false, screen: "main", writing: null };

function plural(n, one, many) {
	return n + " " + (n === 1 ? one : many);
}

// Decimal, as the card's label is: a 64 GB card reads 62.5, not 58.2
// A Pixel 2 card before its first start has no volume to name it by
function cardName(c) {
	return c.started ? c.name : "Pixel 2 card";
}

function size(bytes) {
	return (bytes / 1e9).toFixed(1) + " GB";
}

function selected() {
	return state.cards.find((c) => c.id === state.cardId) || null;
}

function describe(card) {
	if (!card.device) return "No TortOS on this card";
	if (!card.started) return "TortOS for " + DEVICE[card.device] + ", not started yet: its first start makes room for games";
	const which = "TortOS " + (card.version || "before 1.4") + " for " + DEVICE[card.device];
	const counts = [];
	if (card.games) counts.push(plural(card.games, "game", "games"));
	if (card.songs) counts.push(plural(card.songs, "song", "songs"));
	if (card.books) counts.push(plural(card.books, "audiobook", "audiobooks"));
	return which + " · " + (counts.length ? counts.join(", ") : "no games or music yet");
}

// An update only makes sense on a card with this device's TortOS on it
function updateBlocked(card) {
	if (!card) return "Put in a card.";
	if (!card.device) return "There is no TortOS on this card to update.";
	if (card.device !== state.device) return "This card has TortOS for " + DEVICE[card.device] + ".";
	return null;
}

function render() {
	const card = selected();
	const pick = $("card-pick");

	pick.hidden = state.cards.length < 2;
	pick.replaceChildren(...state.cards.map((c) => {
		const o = document.createElement("option");
		o.value = c.id;
		o.textContent = c.name + " · " + size(c.size_bytes);
		o.selected = c.id === state.cardId;
		return o;
	}));

	if (card) {
		$("card-name").textContent = card.name + " · " + size(card.size_bytes);
		$("card-what").textContent = describe(card);
		$("card-what").classList.toggle("found", !!card.device);
	} else {
		$("card-name").textContent = "No card yet";
		$("card-what").textContent = "Put a card in this computer; it shows up here by itself.";
		$("card-what").classList.remove("found");
	}

	for (const b of document.querySelectorAll("[data-group=device] .choice"))
		b.classList.toggle("on", b.dataset.value === state.device);

	const blocked = updateBlocked(card);
	if (blocked && state.action === "update") state.action = "fresh";
	if (!state.action) state.action = blocked ? "fresh" : "update";
	const updateBtn = document.querySelector('[data-group=action] [data-value="update"]');
	updateBtn.disabled = !!blocked;
	$("update-what").textContent = blocked || "Keeps the games, music and saves on the card.";
	for (const b of document.querySelectorAll("[data-group=action] .choice"))
		b.classList.toggle("on", b.dataset.value === state.action);

	const fresh = state.action === "fresh";
	const go = $("go");
	go.textContent = fresh ? "Erase and install" : "Update card";
	go.classList.toggle("erase", fresh);
	go.disabled = !card || !state.device;
	$("note").textContent = !card ? ""
		: !state.device ? "Choose the device this card is for."
		: fresh ? "Everything on the card goes: games, music, saves."
		: "Nothing on the card is erased.";
}

// A new card, or a different one picked: take its device and the action it
// allows, unless the device was chosen by hand for this card
function cardChanged() {
	const card = selected();
	state.userDevice = false;
	state.device = card && card.device ? card.device : state.device;
	state.action = null;
	render();
}

async function refresh() {
	// Only on the main screen: never asking the system about disks mid-write
	if (!invoke || state.screen !== "main") return;
	let cards;
	try { cards = await invoke("list_cards"); } catch (_) { return; }
	const before = state.cardId;
	state.cards = cards;
	if (!cards.some((c) => c.id === state.cardId)) state.cardId = cards.length ? cards[0].id : null;
	if (state.cardId !== before) cardChanged(); else render();
}

for (const group of document.querySelectorAll("[data-group]")) {
	group.addEventListener("click", (e) => {
		const pick = e.target.closest(".choice");
		if (!pick || pick.disabled) return;
		if (group.dataset.group === "device") {
			state.device = pick.dataset.value;
			state.userDevice = true;
			state.action = null;
		} else {
			state.action = pick.dataset.value;
		}
		render();
	});
}
$("card-pick").addEventListener("change", (e) => {
	state.cardId = e.target.value;
	cardChanged();
});

// ---- the screens after the button --------------------------------------

const SCREENS = ["main", "confirm", "working", "finished"];
function show(name) {
	for (const s of SCREENS) $(s).hidden = s !== name;
	state.screen = name;
}

const MB = (n) => Math.round(n / 1e6);

function begin() {
	const card = selected();
	if (!card || !state.device) return;
	if (state.action === "fresh") {
		$("confirm-card").textContent = cardName(card) + " \u00b7 " + size(card.size_bytes);
		$("confirm-device").textContent = DEVICE[state.device];
		show("confirm");
	} else {
		write();
	}
}

async function write() {
	const card = selected();
	state.writing = { card, device: state.device, action: state.action };
	$("working-title").textContent = state.action === "fresh" ? "Installing TortOS" : "Updating the card";
	$("working-phase").textContent = "Opening the card";
	$("working-what").textContent = "Your computer may ask for your password.";
	$("working-bar").style.width = "0";
	$("working-cancel").disabled = false;
	show("working");
	try {
		await invoke("start", { card: card.id, device: state.device, action: state.action });
	} catch (e) {
		finish({ ok: false, cancelled: false, message: String(e) });
	}
}

// Writing is the first half of the bar, checking the second
function progress(p) {
	const half = p.total ? p.done / p.total / 2 : 0;
	$("working-bar").style.width = ((p.phase === "checking" ? 0.5 : 0) + half) * 100 + "%";
	$("working-phase").textContent = (p.phase === "checking" ? "Checking what was written" : "Writing")
		+ " \u00b7 " + MB(p.done) + " of " + MB(p.total) + " MB";
	$("working-what").textContent = p.phase === "checking"
		? "Reading it all back, to be sure the card holds what was written."
		: state.writing.action === "fresh" ? "TortOS and plastron, the whole card." : "TortOS and plastron. The games, music and saves stay.";
}

function finish(f) {
	const w = state.writing || {};
	const head = $("finished-head");
	head.classList.toggle("bad", !f.ok);
	if (f.ok) {
		$("finished-title").textContent = "Done";
		head.textContent = w.action === "fresh" ? "TortOS is on the card." : "The card is updated.";
		$("finished-what").textContent = "Put it in " + DEVICE[w.device] + " and turn it on."
			+ (w.action === "fresh" ? " The first start takes a few seconds longer." : "");
	} else if (f.cancelled) {
		$("finished-title").textContent = "Stopped";
		head.textContent = "The write was stopped partway.";
		$("finished-what").textContent = w.action === "fresh"
			? "The card has no system on it now. Install again to finish."
			: "The card won't start until the update is finished: run it again. The games, music and saves are still there.";
	} else {
		$("finished-title").textContent = "That didn't work";
		head.textContent = f.message || "Something went wrong.";
		$("finished-what").textContent = "";
	}
	state.writing = null;
	show("finished");
}

$("go").addEventListener("click", begin);
$("confirm-back").addEventListener("click", () => show("main"));
$("confirm-go").addEventListener("click", write);
$("working-cancel").addEventListener("click", () => {
	$("working-cancel").disabled = true;
	$("working-phase").textContent = "Stopping";
	invoke("cancel");
});
$("finished-back").addEventListener("click", () => { show("main"); refresh(); });

if (window.__TAURI__) {
	window.__TAURI__.event.listen("progress", (e) => progress(e.payload));
	window.__TAURI__.event.listen("finished", (e) => finish(e.payload));
	window.__TAURI__.app.getVersion().then((v) => { $("ver").textContent = v; });
}
render();
refresh();
setInterval(refresh, 2000);
