// The main screen: the cards in this computer, which device and what to do.
// The cards come from the app (list_cards), every two seconds, so a card put
// in or taken out shows without a button. Writing comes in a later step.

const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : null;
const $ = (id) => document.getElementById(id);
const DEVICE = { brick: "the Brick", pixel2: "the Pixel 2" };

const state = { cards: [], cardId: null, device: null, action: null, userDevice: false };

function plural(n, one, many) {
	return n + " " + (n === 1 ? one : many);
}

// Decimal, as the card's label is: a 64 GB card reads 62.5, not 58.2
function size(bytes) {
	return (bytes / 1e9).toFixed(1) + " GB";
}

function selected() {
	return state.cards.find((c) => c.id === state.cardId) || null;
}

function describe(card) {
	if (!card.device) return "No TortOS on this card";
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
	if (!invoke) return;
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

if (window.__TAURI__) window.__TAURI__.app.getVersion().then((v) => { $("ver").textContent = v; });
render();
refresh();
setInterval(refresh, 2000);
