// Each pair keeps one choice, and the button and note follow what is chosen.
// The card shown is still the mockup's; reading real cards is the next step.
for (const group of document.querySelectorAll("[data-group]")) {
	group.addEventListener("click", (e) => {
		const pick = e.target.closest(".choice");
		if (!pick) return;
		for (const c of group.children) c.classList.toggle("on", c === pick);
		follow();
	});
}
function follow() {
	const fresh = document.querySelector('[data-group=action] .on').dataset.value === "fresh";
	const go = document.getElementById("go");
	go.textContent = fresh ? "Erase and install" : "Update card";
	go.classList.toggle("erase", fresh);
	document.getElementById("note").textContent = fresh
		? "Everything on the card goes: games, music, saves."
		: "About 40 seconds. Nothing on the card is erased.";
}
