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
      if (entry.isIntersecting && entry.target.classList.contains("hero-visual")) {
        entry.target.classList.add("has-entered");
      }
    }
  }, { rootMargin: "100px 0px", threshold: 0 });
  document.querySelectorAll(".hero-visual, .signal-band").forEach((element) => {
    ambientMotion.observe(element);
  });
}

const visual = document.querySelector(".hero-visual");
if (visual && window.matchMedia("(hover: hover) and (pointer: fine)").matches) {
  let bounds;
  let pointerFrame = 0;
  let pointerX = 0;
  let pointerY = 0;
  visual.addEventListener("pointerenter", () => { bounds = visual.getBoundingClientRect(); });
  window.addEventListener("scroll", () => { bounds = undefined; }, { passive: true });
  window.addEventListener("resize", () => { bounds = undefined; }, { passive: true });
  visual.addEventListener("pointermove", (event) => {
    if (reducedMotion.matches) return;
    if (!bounds) bounds = visual.getBoundingClientRect();
    pointerX = event.clientX;
    pointerY = event.clientY;
    if (pointerFrame) return;
    pointerFrame = window.requestAnimationFrame(() => {
      if (!bounds) { pointerFrame = 0; return; }
      const x = (pointerX - bounds.left) / bounds.width - .5;
      const y = (pointerY - bounds.top) / bounds.height - .5;
      visual.style.setProperty("--tilt-x", `${-y * 4}deg`);
      visual.style.setProperty("--tilt-y", `${x * 4}deg`);
      pointerFrame = 0;
    });
  });
  visual.addEventListener("pointerleave", () => {
    window.cancelAnimationFrame(pointerFrame);
    pointerFrame = 0;
    bounds = undefined;
    visual.style.removeProperty("--tilt-x");
    visual.style.removeProperty("--tilt-y");
  });
}
