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
}
