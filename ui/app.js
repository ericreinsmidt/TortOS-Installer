// The installer's screens. Main: the cards in this computer (list_cards, every
// two seconds, so a card put in or taken out shows by itself), which device
// and what to do. Then a confirm before erasing, the write with its progress
// ("progress" and "finished" events from the app), and how it ended.

const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : null;
const $ = (id) => document.getElementById(id);
const DEVICE = { brick: "the Brick", pixel2: "the Pixel 2" };

const state = { cards: [], cardId: null, device: null, action: null, userDevice: false, screen: "main", writing: null,
	latest: null, latestError: null };

// "TortOS 1.3.0", or "TortOS from" the local file a test build comes from
function latestName() {
	return state.latest ? "TortOS " + state.latest : "TortOS";
}

function plural(n, one, many) {
	return n + " " + (n === 1 ? one : many);
}

// Decimal, as the card's label is: a 64 GB card reads 62.5, not 58.2
// A card before its first start has no volume to name it by
function cardName(c) {
	return c.started ? c.name : c.device === "brick" ? "Brick card" : "Pixel 2 card";
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
	// A card whose games partition isn't mounted can't be read: said so, not
	// taken for a TortOS too old to say its version
	if (!card.volume) return "TortOS for " + DEVICE[card.device] + " · its games partition isn't mounted, so what's on it can't be read";
	const which = "TortOS " + (card.version || "before 1.4") + " for " + DEVICE[card.device]
		+ (card.trimui ? ", on TrimUI's firmware" : "");
	const counts = [];
	if (card.games) counts.push(plural(card.games, "game", "games"));
	if (card.songs) counts.push(plural(card.songs, "song", "songs"));
	if (card.books) counts.push(plural(card.books, "audiobook", "audiobooks"));
	return which + " · " + (counts.length ? counts.join(", ") : "no games or music yet");
}

// An update only makes sense on a card with this device's TortOS on it, on
// its own system
function updateBlocked(card) {
	if (!card) return "Put in a card.";
	if (!card.device) return "There is no TortOS on this card to update.";
	if (card.device !== state.device) return "This card has TortOS for " + DEVICE[card.device] + ".";
	if (card.trimui) return "This card runs TrimUI's firmware: moving it to TortOS 2 erases it.";
	return null;
}

function render() {
	const card = selected();
	const pick = $("card-pick");

	pick.hidden = state.cards.length < 2;
	pick.replaceChildren(...state.cards.map((c) => {
		const o = document.createElement("option");
		o.value = c.id;
		o.textContent = cardName(c) + " · " + size(c.size_bytes);
		o.selected = c.id === state.cardId;
		return o;
	}));

	if (card) {
		$("card-name").textContent = cardName(card) + " · " + size(card.size_bytes);
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
	// A Brick card on TrimUI's firmware has no fresh install of its own: the
	// same choice moves it, keeping its saves and settings, so it says so
	const movable = card && card.trimui && state.device === "brick";
	$("fresh-title").textContent = movable ? "Move to TortOS 2" : "Fresh install";
	$("fresh-what").textContent = movable ? "Keeps your saves, settings and BIOS files; the rest of the card is erased."
		: "Erases the whole card and starts over.";

	const fresh = state.action === "fresh";
	const moving = fresh && card && card.trimui && state.device === "brick";
	const go = $("go");
	go.textContent = moving ? "Move to TortOS 2" : fresh ? "Erase and install" : "Update card";
	go.classList.toggle("erase", fresh);
	go.disabled = !card || !state.device;
	const already = !fresh && card && state.latest && card.version === state.latest;
	$("note").textContent = !card ? ""
		: !state.device ? "Choose the device this card is for."
		: state.latestError && !state.latest ? "GitHub can't be reached right now. TortOS is downloaded from there."
		: already ? "This card already has " + latestName() + ": updating writes it again."
		: moving ? latestName() + " \u00b7 The card is erased. Your saves and settings are kept on this computer and put back after."
		: latestName() + " \u00b7 " + (fresh ? "Everything on the card goes: games, music, saves." : "Nothing on the card is erased.");

	// A card moved over, back after its first start, with its saves and
	// settings waiting on this computer
	const waiting = card && card.backup;
	$("restore").hidden = !waiting;
	if (waiting) $("restore-head").textContent = "Your saves and settings from " + when(card.backup.made) + " are ready to go back on this card.";
}

// A backup's time, in this computer's own time
function when(seconds) {
	return new Date(seconds * 1000).toLocaleString(undefined, { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
}

// The backup folder's name, in this computer's own time: Brick 2026-10-24 1405
function backupLabel() {
	const d = new Date();
	const two = (n) => String(n).padStart(2, "0");
	return "Brick " + d.getFullYear() + "-" + two(d.getMonth() + 1) + "-" + two(d.getDate()) + " " + two(d.getHours()) + two(d.getMinutes());
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
	// On the main screen, and on the Done screen after a move, which waits for
	// the card to come back from the Brick's first start; never mid-write
	const awaiting = state.screen === "finished" && state.awaitRestore;
	if (!invoke || (state.screen !== "main" && !awaiting)) return;
	let cards;
	try { cards = await invoke("list_cards"); } catch (_) { return; }
	if (awaiting) {
		// The moved card back with its saves and settings waiting: straight to
		// the main screen, where Put them back is, with no Back to press
		const back = cards.find((c) => c.backup);
		if (!back || state.screen !== "finished") return;
		state.awaitRestore = false;
		state.cards = cards;
		state.cardId = back.id;
		show("main");
		cardChanged();
		return;
	}
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

async function begin() {
	const card = selected();
	if (!card || !state.device) return;
	if (state.action !== "fresh") return write();
	const moving = card.trimui && state.device === "brick";
	$("confirm-title").textContent = moving ? "Move to TortOS 2" : "Fresh install";
	$("confirm-card").textContent = cardName(card) + " \u00b7 " + size(card.size_bytes);
	$("confirm-device").textContent = DEVICE[state.device];
	$("confirm-what").textContent = moving
		? "This erases the whole card: TortOS 2 brings its own system, in place of TrimUI's firmware."
		: "Everything on it goes: games, music, saves.";
	const yours = [];
	if (card.games) yours.push(plural(card.games, "game", "games"));
	if (card.songs) yours.push(plural(card.songs, "song", "songs"));
	if (card.books) yours.push(plural(card.books, "audiobook", "audiobooks"));
	$("confirm-backup").hidden = !moving;
	$("confirm-yours").hidden = !moving || !yours.length;
	$("confirm-check-row").hidden = !moving || !yours.length;
	$("confirm-check").checked = false;
	state.planFailed = false;
	$("confirm-go").textContent = moving ? "Back up and erase" : "Erase and install";
	$("confirm-go").disabled = moving && yours.length > 0;
	if (moving) {
		$("confirm-backup").textContent = "Finding what to back up\u2026";
		$("confirm-yours").textContent = "Copy these to your computer yourself before going on: " + yours.join(", ") + ". They are erased with the card.";
		invoke("backup_plan", { card: card.id }).then(([bytes, folder]) => {
			$("confirm-backup").textContent = "Your saves, settings and BIOS files (" + MB(bytes) + " MB) are copied to "
				+ folder + " first, and put back after the Brick's first start.";
		}, (e) => { $("confirm-backup").textContent = String(e); state.planFailed = true; $("confirm-go").disabled = true; });
	}
	show("confirm");
}

async function write() {
	const card = selected();
	const moving = state.action === "fresh" && card.trimui && state.device === "brick";
	const action = moving ? "move" : state.action;
	state.writing = { card, device: state.device, action };
	$("working-title").textContent = moving ? "Moving to TortOS 2" : state.action === "fresh" ? "Installing TortOS" : "Updating the card";
	const local = state.latest && state.latest.startsWith("from ");
	$("working-phase").textContent = moving ? "Backing up" : !local ? "Getting " + latestName() : "Opening the card";
	$("working-what").textContent = moving ? WRITING.move : "Your computer may ask for your password.";
	$("working-bar").style.width = "0";
	$("working-cancel").disabled = false;
	show("working");
	try {
		await invoke("start", { card: card.id, device: state.device, action, label: moving ? backupLabel() : null });
	} catch (e) {
		finish({ ok: false, cancelled: false, message: String(e) });
	}
}

// What is being written, by action: the same on both devices
const WRITING = {
	fresh: "TortOS and plastron, the whole card.",
	update: "TortOS and plastron. The games, music and saves stay.",
	move: "Your saves, settings and BIOS files, to this computer: each file read back.",
	restore: "Your saves, settings and BIOS files, back onto the card: each file read back.",
};

async function restore() {
	const card = selected();
	if (!card || !card.backup) return;
	state.writing = { card, device: "brick", action: "restore" };
	$("working-title").textContent = "Putting back your saves and settings";
	$("working-phase").textContent = "Putting back";
	$("working-what").textContent = WRITING.restore;
	$("working-bar").style.width = "0";
	$("working-cancel").disabled = false;
	show("working");
	try {
		await invoke("start", { card: card.id, device: "brick", action: "restore", label: null });
	} catch (e) {
		finish({ ok: false, cancelled: false, message: String(e) });
	}
}

// Writing is the first half of the bar, checking the second
function progress(p) {
	const w = state.writing;
	if (p.phase === "ejecting") {
		$("working-phase").textContent = "Ejecting the card";
		$("working-what").textContent = "So it can come out safely.";
		$("working-cancel").disabled = true;
		return;
	}
	if (p.phase === "downloading") {
		$("working-bar").style.width = (p.total ? p.done / p.total : 0) * 100 + "%";
		$("working-phase").textContent = "Downloading " + latestName() + " \u00b7 " + MB(p.done) + " of " + MB(p.total) + " MB";
		$("working-what").textContent = "From GitHub, then checked against the release's checksums.";
		return;
	}
	const part = p.total ? p.done / p.total : 0;
	if (p.phase === "copying") {
		// Before a move's write: nothing on the card is changed yet
		if (w.action === "restore") w.wrote = true;
		$("working-bar").style.width = part * 100 + "%";
		$("working-phase").textContent = (w.action === "restore" ? "Putting back" : "Backing up")
			+ " \u00b7 " + MB(p.done) + " of " + MB(p.total) + " MB";
		$("working-what").textContent = WRITING[w.action];
		return;
	}
	w.wrote = true;
	$("working-bar").style.width = ((p.phase === "checking" ? 0.5 : 0) + part / 2) * 100 + "%";
	$("working-phase").textContent = (p.phase === "checking" ? "Checking what was written" : "Writing")
		+ " \u00b7 " + MB(p.done) + " of " + MB(p.total) + " MB";
	$("working-what").textContent = p.phase === "checking"
		? "Reading it all back, to be sure the card holds what was written."
		: WRITING[w.action === "move" ? "fresh" : w.action];
}

function finish(f) {
	const w = state.writing || {};
	const head = $("finished-head");
	head.classList.toggle("bad", !f.ok);
	if (f.ok && w.action === "move") {
		$("finished-title").textContent = "Done";
		head.textContent = latestName() + " is on the card. Your saves and settings are in " + f.backup + ".";
		$("finished-what").textContent = "Put the card in the Brick and turn it on. When it reaches the shelf, turn it off "
			+ "and put the card back in this computer, and this window offers to put your saves and settings back.";
	} else if (f.ok && w.action === "restore") {
		// The card stays in: the games, music and audiobooks go back on next
		$("finished-title").textContent = "Done";
		head.textContent = "Your saves and settings are back on the card.";
		$("finished-what").textContent = "Now copy your games, music and audiobooks back onto the card: it's TORTOS on this "
			+ "computer. Then eject it, put it in the Brick and turn it on. The backup stays on this computer until you delete it.";
	} else if (f.ok) {
		$("finished-title").textContent = "Done";
		head.textContent = w.action === "fresh" ? latestName() + " is on the card." : "The card is updated to " + latestName() + ".";
		$("finished-what").textContent = "Put it in " + DEVICE[w.device] + " and turn it on."
			+ (w.action === "fresh" ? " The first start takes a few seconds longer." : "");
	} else if (f.cancelled && !w.wrote) {
		$("finished-title").textContent = "Stopped";
		head.textContent = "Stopped before anything was written.";
		$("finished-what").textContent = "The card is as it was.";
	} else if (f.cancelled) {
		$("finished-title").textContent = "Stopped";
		head.textContent = "The write was stopped partway.";
		$("finished-what").textContent = w.action === "restore"
			? "Put it back again to finish: the installer offers it until it's done."
			: w.action !== "update"
			? "The card has no system on it now. Install again to finish."
			: "The card won't start until the update is finished: run it again. The games, music and saves are still there.";
	} else {
		$("finished-title").textContent = "That didn't work";
		head.textContent = f.message || "Something went wrong.";
		$("finished-what").textContent = "";
	}
	const kept = f.ok && w.action === "restore";
	$("finished-show").hidden = !kept;
	$("finished-eject").hidden = !kept;
	$("finished-back").hidden = kept;
	state.keptCard = kept ? w.card.id : null;
	// Ejected after a job that finished well, or why not: said first
	if (f.ok && !kept) {
		$("finished-what").textContent = (f.ejected ? "The card is ejected: you can take it out. "
			: "Eject the card before taking it out. " + (f.eject_error ? f.eject_error + " " : ""))
			+ $("finished-what").textContent;
	}
	state.awaitRestore = f.ok && w.action === "move";
	state.writing = null;
	show("finished");
}

$("go").addEventListener("click", begin);
$("confirm-back").addEventListener("click", () => show("main"));
$("confirm-go").addEventListener("click", write);
$("confirm-check").addEventListener("change", (e) => { $("confirm-go").disabled = !e.target.checked || state.planFailed; });
$("restore-go").addEventListener("click", restore);
$("working-cancel").addEventListener("click", () => {
	$("working-cancel").disabled = true;
	$("working-phase").textContent = "Stopping";
	invoke("cancel");
});
$("finished-back").addEventListener("click", () => { state.awaitRestore = false; show("main"); refresh(); });
$("finished-show").addEventListener("click", () => {
	invoke("show_card", { card: state.keptCard }).catch((e) => { $("finished-what").textContent = String(e); });
});
$("finished-eject").addEventListener("click", async () => {
	$("finished-eject").disabled = true;
	try {
		await invoke("eject_card", { card: state.keptCard });
		$("finished-what").textContent = "The card is ejected: you can take it out. Put it in the Brick and turn it on. "
			+ "The backup stays on this computer until you delete it.";
		$("finished-show").hidden = true;
		$("finished-eject").hidden = true;
		$("finished-back").hidden = false;
		state.keptCard = null;
	} catch (e) {
		$("finished-what").textContent = "The card couldn't be ejected: " + e + " Close what has it open and try again.";
	}
	$("finished-eject").disabled = false;
});

if (window.__TAURI__) {
	window.__TAURI__.event.listen("progress", (e) => progress(e.payload));
	window.__TAURI__.event.listen("finished", (e) => finish(e.payload));
	window.__TAURI__.app.getVersion().then((v) => { $("ver").textContent = v; });
	findLatest();
	invoke("newer_installer").then((v) => {
		if (!v) return;
		const link = $("newer");
		link.textContent = "Installer " + v + " is out";
		link.hidden = false;
	});
}

// What TortOS the button installs, asked once; again every half minute while
// GitHub can't be reached
async function findLatest() {
	try {
		state.latest = await invoke("latest_tortos");
		state.latestError = null;
	} catch (e) {
		state.latestError = String(e);
		setTimeout(findLatest, 30000);
	}
	if (state.screen === "main") render();
}

$("newer").addEventListener("click", () => invoke("open_installer_releases"));
render();
refresh();
setInterval(refresh, 2000);
