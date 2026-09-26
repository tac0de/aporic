const toggle = document.querySelector(".menu-toggle");
const nav = document.querySelector(".primary-nav");

if (toggle && nav) {
  const closeMenu = () => {
    toggle.setAttribute("aria-expanded", "false");
    toggle.setAttribute("aria-label", "Open navigation");
    nav.classList.remove("is-open");
  };

  toggle.addEventListener("click", () => {
    const expanded = toggle.getAttribute("aria-expanded") === "true";
    toggle.setAttribute("aria-expanded", String(!expanded));
    toggle.setAttribute("aria-label", expanded ? "Open navigation" : "Close navigation");
    nav.classList.toggle("is-open", !expanded);
  });

  nav.addEventListener("click", (event) => {
    if (event.target.closest("a")) closeMenu();
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") closeMenu();
  });

  document.addEventListener("click", (event) => {
    if (!event.target.closest(".site-header")) closeMenu();
  });
}

const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
const progress = document.querySelector(".reading-progress span");
let scrollQueued = false;

const flow = document.querySelector(".hero-visual");
if (flow) {
  const frame = flow.querySelector(".flow-frame");
  const steps = [...flow.querySelectorAll("[data-flow-to]")];
  const cards = [...flow.querySelectorAll("[data-flow-panel]")];
  const previous = flow.querySelector(".flow-back");
  const next = flow.querySelector(".flow-next");
  const announcement = flow.querySelector(".flow-announcement");
  let active = 0;

  const selectStep = (index, announce = true) => {
    active = Math.max(0, Math.min(index, cards.length - 1));
    frame.style.setProperty("--flow-progress", String((active + 1) / cards.length));
    flow.querySelector(".flow-count").textContent = `${String(active + 1).padStart(2, "0")} / ${String(cards.length).padStart(2, "0")}`;
    steps.forEach((button, position) => {
      button.setAttribute("aria-pressed", String(position === active));
    });
    cards.forEach((card, position) => {
      const selected = position === active;
      card.classList.toggle("is-active", selected);
      card.setAttribute("aria-hidden", String(!selected));
      card.inert = !selected;
    });
    previous.disabled = active === 0;
    next.textContent = active === cards.length - 1 ? "Start again ↺" : "Next step →";
    if (announce) {
      const label = steps[active].querySelector("span").textContent;
      announcement.textContent = `${label}. ${cards[active].querySelector("h2").textContent} ${cards[active].querySelector(".flow-card-persist strong").textContent} stays with the project.`;
    }
  };

  flow.classList.add("flow-ready");
  steps.forEach((button, index) => {
    button.addEventListener("click", () => selectStep(index));
    button.addEventListener("keydown", (event) => {
      const target = event.key === "ArrowRight" ? Math.min(index + 1, steps.length - 1)
        : event.key === "ArrowLeft" ? Math.max(index - 1, 0) : null;
      if (target === null) return;
      event.preventDefault();
      steps[target].focus();
      selectStep(target);
    });
  });
  previous.addEventListener("click", () => selectStep(active - 1));
  next.addEventListener("click", () => selectStep((active + 1) % cards.length));
  selectStep(0, false);
}

const updateProgress = () => {
  const available = document.documentElement.scrollHeight - window.innerHeight;
  const fraction = available > 0 ? Math.min(1, window.scrollY / available) : 0;
  if (progress) progress.style.transform = `scaleX(${fraction})`;
  scrollQueued = false;
};

window.addEventListener("scroll", () => {
  if (scrollQueued) return;
  scrollQueued = true;
  window.requestAnimationFrame(updateProgress);
}, { passive: true });
window.addEventListener("resize", updateProgress, { passive: true });
updateProgress();

if ("IntersectionObserver" in window && !reducedMotion.matches) {
  document.documentElement.classList.add("motion-ready");
  const reveal = new IntersectionObserver((entries, observer) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      entry.target.classList.add("is-visible");
      observer.unobserve(entry.target);
    }
  }, { threshold: .12, rootMargin: "0px 0px -6% 0px" });
  document.querySelectorAll("[data-reveal]").forEach((element) => reveal.observe(element));

  const ambientMotion = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      entry.target.classList.toggle("is-playing", entry.isIntersecting);
    }
  }, { rootMargin: "100px 0px", threshold: 0 });
  document.querySelectorAll(".hero-visual, .signal-band").forEach((element) => {
    ambientMotion.observe(element);
  });
}
