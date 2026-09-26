const proposals = [
  {
    id: "orientation",
    title: "A clearer first step",
    summary: "A focused opening screen that gives new visitors one useful route into the workspace.",
    evidence: "Open the first screen, follow its main action, and check whether the next step is obvious.",
  },
  {
    id: "comparison",
    title: "Compare two directions",
    summary: "A side-by-side view that keeps the criteria visible while a team weighs alternatives.",
    evidence: "Compare the same criteria in both columns and check that the smaller screen keeps their meaning.",
  },
  {
    id: "handoff",
    title: "A calmer handoff",
    summary: "A compact summary that helps the next reviewer pick up where the last one stopped.",
    evidence: "Read the handoff and identify the completed work, open question, and next action.",
  },
];

const reviewed = new Set();
const cards = document.querySelector("#cards");
const emptyState = document.querySelector("#empty-state");
const progressCount = document.querySelector("#progress-count");
const status = document.querySelector("#board-status");
const dialog = document.querySelector("#review-dialog");
const form = document.querySelector("#review-form");
const completeButton = document.querySelector("#complete-review");
let filter = "all";
let activeProposal = null;
let opener = null;

function visibleProposals() {
  return proposals.filter((proposal) => {
    if (filter === "reviewed") return reviewed.has(proposal.id);
    if (filter === "pending") return !reviewed.has(proposal.id);
    return true;
  });
}

function render() {
  const visible = visibleProposals();
  cards.replaceChildren(...visible.map((proposal) => {
    const card = document.createElement("article");
    card.className = "proposal-card";
    const isReviewed = reviewed.has(proposal.id);
    card.innerHTML = `
      <div class="card-top">
        <span class="card-index">PROPOSAL ${String(proposals.indexOf(proposal) + 1).padStart(2, "0")}</span>
        <span class="status-pill ${isReviewed ? "reviewed" : ""}">${isReviewed ? "Reviewed" : "To review"}</span>
      </div>
      <h3></h3>
      <p></p>
      <button class="card-action" type="button" aria-label="Review ${proposal.title}">${isReviewed ? "Review again" : "Open review"}<span aria-hidden="true">↗</span></button>
    `;
    card.querySelector("h3").textContent = proposal.title;
    card.querySelector("p").textContent = proposal.summary;
    card.querySelector("button").addEventListener("click", (event) => openReview(proposal, event.currentTarget));
    return card;
  }));
  emptyState.hidden = visible.length !== 0;
  progressCount.textContent = `${reviewed.size} / ${proposals.length}`;
  status.textContent = `${visible.length} proposals shown. ${reviewed.size} of ${proposals.length} reviewed.`;
}

function openReview(proposal, button) {
  activeProposal = proposal;
  opener = button;
  form.reset();
  completeButton.disabled = true;
  document.querySelector("#dialog-title").textContent = proposal.title;
  document.querySelector("#dialog-summary").textContent = proposal.summary;
  document.querySelector("#dialog-evidence").textContent = proposal.evidence;
  dialog.showModal();
  form.elements.flow.focus();
}

form.addEventListener("change", () => {
  completeButton.disabled = !["flow", "narrow", "keyboard"].every((name) => form.elements[name].checked);
});

form.addEventListener("submit", (event) => {
  event.preventDefault();
  if (completeButton.disabled || !activeProposal) return;
  const completedProposal = activeProposal;
  reviewed.add(completedProposal.id);
  dialog.close();
  render();
  const replacement = [...cards.querySelectorAll(".card-action")]
    .find((button) => button.getAttribute("aria-label") === `Review ${completedProposal.title}`);
  if (replacement) replacement.focus();
  else document.querySelector('[data-filter="all"]').focus();
});

function closeReview() { dialog.close(); }
document.querySelector("#close-dialog").addEventListener("click", closeReview);
document.querySelector("#cancel-review").addEventListener("click", closeReview);
dialog.addEventListener("close", () => {
  if (!reviewed.has(activeProposal?.id) && opener?.isConnected) opener.focus();
  activeProposal = null;
  opener = null;
});

document.querySelectorAll("[data-filter]").forEach((button) => {
  button.addEventListener("click", () => {
    filter = button.dataset.filter;
    document.querySelectorAll("[data-filter]").forEach((item) => {
      const active = item === button;
      item.classList.toggle("is-active", active);
      item.setAttribute("aria-pressed", String(active));
    });
    render();
  });
});

render();
