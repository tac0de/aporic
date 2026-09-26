const proposals = {
  '01': { title: 'Quiet confidence', summary: 'Clear typography and considered space create a brand that speaks softly, yet stays with you.' },
  '02': { title: 'Open by nature', summary: 'A lively, modular experience designed to make discovery feel effortless and inviting.' },
  '03': { title: 'Made of moments', summary: 'A human-centered campaign built from small encounters and stories worth sharing.' }
};

const reviewed = new Set();
const dialog = document.querySelector('#review-dialog');
const checks = [...dialog.querySelectorAll('input[type="checkbox"]')];
const markButton = document.querySelector('#mark-reviewed');
const cards = [...document.querySelectorAll('.proposal-card')];
const filters = [...document.querySelectorAll('.filter')];
let activeId = null;
let activeFilter = 'all';
let returnFocus = null;

function updateBoard() {
  const count = reviewed.size;
  document.querySelector('#completed-count').textContent = count;
  document.querySelector('#progress-fill').style.width = `${(count / 3) * 100}%`;
  document.querySelector('.progress-track').setAttribute('aria-valuenow', String(count));
  document.querySelector('#progress-message').textContent = count === 3 ? 'All proposals reviewed. Thank you.' : count === 0 ? 'A fresh perspective starts here.' : `${3 - count} ${3 - count === 1 ? 'proposal' : 'proposals'} left to explore.`;
  document.querySelector('#todo-count').textContent = String(3 - count).padStart(2, '0');
  document.querySelector('#reviewed-count').textContent = String(count).padStart(2, '0');

  let visible = 0;
  cards.forEach(card => {
    const isReviewed = reviewed.has(card.dataset.id);
    card.dataset.status = isReviewed ? 'reviewed' : 'todo';
    card.querySelector('.status').innerHTML = `<span class="status-dot"></span>${isReviewed ? 'Reviewed' : 'To review'}`;
    card.hidden = activeFilter === 'reviewed' ? !isReviewed : activeFilter === 'todo' ? isReviewed : false;
    if (!card.hidden) visible += 1;
  });
  document.querySelector('#empty-state').hidden = visible !== 0;
}

function updateDialog() {
  const done = reviewed.has(activeId);
  markButton.hidden = done;
  markButton.disabled = !checks.every(check => check.checked);
  document.querySelector('#dialog-hint').textContent = done ? 'You have already reviewed this proposal.' : 'Complete all three checks to continue.';
}

function openProposal(id, opener) {
  activeId = id;
  returnFocus = opener;
  document.querySelector('#dialog-number').textContent = `DIRECTION ${id}`;
  document.querySelector('#dialog-title').textContent = proposals[id].title;
  document.querySelector('#dialog-summary').textContent = proposals[id].summary;
  checks.forEach(check => { check.checked = reviewed.has(id); check.disabled = reviewed.has(id); });
  updateDialog();
  dialog.showModal();
  document.querySelector('#dialog-close').focus();
}

cards.forEach(card => card.querySelector('.open-proposal').addEventListener('click', event => openProposal(card.dataset.id, event.currentTarget)));
checks.forEach(check => check.addEventListener('change', updateDialog));
document.querySelector('#dialog-close').addEventListener('click', () => dialog.close());
dialog.addEventListener('close', () => {
  if (returnFocus?.isConnected && !returnFocus.closest('[hidden]')) returnFocus.focus();
  else document.querySelector(`.filter[data-filter="${activeFilter}"]`).focus();
});
markButton.addEventListener('click', () => {
  if (markButton.disabled || !activeId) return;
  reviewed.add(activeId);
  updateBoard();
  dialog.close();
});
filters.forEach(filter => filter.addEventListener('click', () => {
  activeFilter = filter.dataset.filter;
  filters.forEach(button => { const active = button === filter; button.classList.toggle('is-active', active); button.setAttribute('aria-pressed', String(active)); });
  updateBoard();
}));
updateBoard();
