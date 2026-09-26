const proposals = [
  { id: '01', title: 'A quieter start', summary: 'Give new members a calmer first week with a guided path to the essentials.', checks: ['Purpose and audience are clear', 'First-week steps feel achievable', 'Success can be measured'] },
  { id: '02', title: 'Signals, not noise', summary: 'Bring the most useful project updates into one thoughtful weekly digest.', checks: ['The update has a clear owner', 'The signal is useful to readers', 'The delivery rhythm is realistic'] },
  { id: '03', title: 'Room to reflect', summary: 'Make space for short retrospectives that turn everyday observations into better work.', checks: ['The reflection prompt is specific', 'Participation feels accessible', 'Follow-up actions have a home'] }
];

const reviewed = new Set();
const list = document.querySelector('#proposal-list');
const dialog = document.querySelector('#review-dialog');
const form = document.querySelector('#review-form');
const markButton = document.querySelector('#mark-reviewed');
let activeFilter = 'all';
let activeProposal = null;
let returnFocus = null;

function render() {
  const visible = proposals.filter(proposal => activeFilter === 'all' || (activeFilter === 'reviewed' ? reviewed.has(proposal.id) : !reviewed.has(proposal.id)));
  list.replaceChildren(...visible.map(proposal => {
    const article = document.createElement('article');
    article.className = 'proposal-card';
    const status = reviewed.has(proposal.id) ? 'Reviewed' : 'To review';
    article.innerHTML = `<div class="card-top"><span class="card-number">PROPOSAL / ${proposal.id}</span><span class="status ${reviewed.has(proposal.id) ? 'reviewed' : ''}">${status}</span></div><h3>${proposal.title}</h3><p>${proposal.summary}</p>`;
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'card-button';
    button.innerHTML = `${reviewed.has(proposal.id) ? 'View review' : 'Review proposal'} <span aria-hidden="true">↗</span>`;
    button.setAttribute('aria-label', `${reviewed.has(proposal.id) ? 'View review for' : 'Review'} ${proposal.title}`);
    button.addEventListener('click', () => openReview(proposal, button));
    article.append(button);
    return article;
  }));
  document.querySelector('#empty-state').hidden = visible.length !== 0;
  document.querySelector('#progress-count').textContent = reviewed.size;
  document.querySelector('.progress-track').setAttribute('aria-valuenow', reviewed.size);
  document.querySelector('#progress-fill').style.width = `${reviewed.size / proposals.length * 100}%`;
  document.querySelector('#progress-description').textContent = reviewed.size === 3 ? 'All proposals have been reviewed.' : `${proposals.length - reviewed.size} proposal${proposals.length - reviewed.size === 1 ? '' : 's'} await your review.`;
  document.querySelector('#pending-count').textContent = String(proposals.length - reviewed.size).padStart(2, '0');
  document.querySelector('#reviewed-count').textContent = String(reviewed.size).padStart(2, '0');
}

function openReview(proposal, trigger) {
  activeProposal = proposal;
  returnFocus = trigger;
  document.querySelector('#dialog-number').textContent = `/ ${proposal.id}`;
  document.querySelector('#dialog-title').textContent = proposal.title;
  document.querySelector('#dialog-summary').textContent = proposal.summary;
  const checklist = document.querySelector('#checklist');
  checklist.replaceChildren(...proposal.checks.map((check, index) => {
    const label = document.createElement('label');
    label.className = 'check-row';
    const input = document.createElement('input');
    input.type = 'checkbox';
    input.name = `check-${index}`;
    input.checked = reviewed.has(proposal.id);
    input.disabled = reviewed.has(proposal.id);
    input.addEventListener('change', updateSubmit);
    label.append(input, document.createTextNode(check));
    return label;
  }));
  markButton.hidden = reviewed.has(proposal.id);
  updateSubmit();
  dialog.showModal();
  document.querySelector('#close-dialog').focus();
}

function updateSubmit() {
  markButton.disabled = reviewed.has(activeProposal?.id) || ![...form.querySelectorAll('input[type="checkbox"]')].every(input => input.checked);
}

function closeReview() { dialog.close(); }
document.querySelector('#close-dialog').addEventListener('click', closeReview);
document.querySelector('#cancel-dialog').addEventListener('click', closeReview);
dialog.addEventListener('close', () => {
  const focusTarget = returnFocus;
  activeProposal = null;
  returnFocus = null;
  if (focusTarget?.isConnected) focusTarget.focus();
  else document.querySelector(`.filter[data-filter="${activeFilter}"]`).focus();
});
form.addEventListener('submit', event => {
  event.preventDefault();
  if (markButton.disabled || !activeProposal) return;
  reviewed.add(activeProposal.id);
  closeReview();
  render();
});
document.querySelectorAll('.filter').forEach(button => button.addEventListener('click', () => {
  activeFilter = button.dataset.filter;
  document.querySelectorAll('.filter').forEach(item => {
    const selected = item === button;
    item.classList.toggle('is-active', selected);
    item.setAttribute('aria-pressed', selected);
  });
  render();
}));
render();
