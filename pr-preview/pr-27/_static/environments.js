// The Development menu: every docs environment on the site (the live site, the preview of main and
// one preview per open pull request), read from environments.json at the site root, which
// tools/gh-pages.sh keeps current. Each links to this page in that environment, or to its front
// page if this page isn't there. Also fills #sg-environments on the Development page.
"use strict";

(() => {
  const script = document.currentScript;
  const SITE = script?.dataset.siteRoot ?? "";
  const MENU_TITLE = "Development";

  // This build's root URL, and this page's path below it.
  const contentRoot = document.documentElement.dataset.content_root ?? "./";
  const buildRoot = new URL(contentRoot, window.location.href).href;
  const page = window.location.href.startsWith(buildRoot)
    ? window.location.href.slice(buildRoot.length).split("#")[0]
    : "";

  function short(sha) {
    return typeof sha === "string" ? sha.slice(0, 7) : "";
  }

  function label(env) {
    if (env.kind === "release") {
      return { title: "Live site", summary: `Release ${env.title ?? ""}`.trim() };
    }
    if (env.kind === "main") {
      return { title: "main", summary: `Preview of main ${short(env.sha)}`.trim() };
    }
    const title = env.title ? `#${env.number} ${env.title}` : `#${env.number}`;
    const branch = env.ref ? `${env.ref} ` : "";
    return { title, summary: `Pull request preview: ${branch}${short(env.sha)}`.trim() };
  }

  function rootOf(env) {
    return SITE + env.path;
  }

  // Go to this page in `env`, or to its front page if this page isn't there.
  function visit(event, env) {
    if (page === "" || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey) {
      return;
    }
    event.preventDefault();
    const target = rootOf(env) + page;
    fetch(target, { method: "HEAD" })
      .then((response) => (response.ok ? target : rootOf(env)))
      .catch(() => rootOf(env))
      .then((url) => window.location.assign(url));
  }

  function link(env, withSummary) {
    const { title, summary } = label(env);
    const a = document.createElement("a");
    a.href = rootOf(env) + page;
    a.addEventListener("click", (event) => visit(event, env));
    if (rootOf(env) === buildRoot) {
      a.setAttribute("aria-current", "page");
      a.classList.add("sg-env-current");
    }
    const name = document.createElement("span");
    name.textContent = title;
    a.append(name);
    if (withSummary) {
      const small = document.createElement("small");
      small.textContent = summary;
      a.append(small);
    }
    return a;
  }

  function fillMenus(environments) {
    for (const trigger of document.querySelectorAll(".js-menu")) {
      if (trigger.textContent.trim() !== MENU_TITLE) {
        continue;
      }
      const list = document.getElementById(trigger.getAttribute("aria-controls") ?? "");
      if (list === null) {
        continue;
      }
      list.replaceChildren(
        ...environments.map((env) => {
          const item = document.createElement("li");
          item.append(link(env, true));
          return item;
        }),
      );
    }
  }

  function fillPage(environments) {
    const container = document.getElementById("sg-environments");
    if (container === null) {
      return;
    }
    const list = document.createElement("ul");
    for (const env of environments) {
      const item = document.createElement("li");
      item.append(link(env, false), ` — ${label(env).summary}`);
      if (env.link) {
        const source = document.createElement("a");
        source.href = env.link;
        source.textContent = env.kind === "pr" ? "pull request" : "source";
        item.append(" (", source, ")");
      }
      list.append(item);
    }
    container.replaceChildren(list);
  }

  fetch(`${SITE}environments.json`, { cache: "no-cache" })
    .then((response) => (response.ok ? response.json() : Promise.reject(response.status)))
    .then((index) => {
      const environments = Array.isArray(index.environments) ? index.environments : [];
      fillMenus(environments);
      fillPage(environments);
    })
    .catch(() => {
      // Offline or not deployed yet: the menu keeps its fixed links.
    });
})();
