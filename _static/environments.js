"use strict";
(() => {
  // src/docs/environments.ts
  var MENU_TITLE = "Development";
  var KINDS = /* @__PURE__ */ new Set(["release", "main", "pr"]);
  function isObject(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
  }
  function text(value) {
    return typeof value === "string" && value !== "" ? value : void 0;
  }
  function parseEnvironment(value) {
    if (!isObject(value)) {
      return void 0;
    }
    const { kind, path, number } = value;
    if (typeof path !== "string" || typeof kind !== "string" || !KINDS.has(kind)) {
      return void 0;
    }
    const title = text(value["title"]);
    const sha = text(value["sha"]);
    const ref = text(value["ref"]);
    const link = text(value["link"]);
    return {
      kind,
      path,
      ...title !== void 0 && { title },
      ...sha !== void 0 && { sha },
      ...typeof number === "number" && { number },
      ...ref !== void 0 && { ref },
      ...link !== void 0 && { link }
    };
  }
  function parseEnvironments(value) {
    const list = isObject(value) ? value["environments"] : void 0;
    return Array.isArray(list) ? list.map((item) => parseEnvironment(item)).filter((item) => item !== void 0) : [];
  }
  function short(sha) {
    return sha === void 0 ? "" : sha.slice(0, 7);
  }
  function label(environment) {
    if (environment.kind === "release") {
      return {
        title: "Live site",
        summary: `Release ${environment.title ?? ""}`.trim()
      };
    }
    if (environment.kind === "main") {
      return { title: "main", summary: `Preview of main ${short(environment.sha)}`.trim() };
    }
    const number = environment.number === void 0 ? "" : `#${String(environment.number)}`;
    const title = environment.title === void 0 ? number : `${number} ${environment.title}`;
    const branch = environment.ref === void 0 ? "" : `${environment.ref} `;
    return {
      title,
      summary: `Pull request preview: ${branch}${short(environment.sha)}`.trim()
    };
  }
  async function pageIn(root, page) {
    try {
      const response = await fetch(root + page, { method: "HEAD" });
      return response.ok ? root + page : root;
    } catch {
      return root;
    }
  }
  async function fetchIndex(site) {
    try {
      const response = await fetch(`${site}environments.json`, { cache: "no-cache" });
      return response.ok ? await response.json() : void 0;
    } catch {
      return void 0;
    }
  }
  async function start(site) {
    const here = location.href;
    const contentRoot = document.documentElement.dataset["content_root"] ?? "./";
    const buildRoot = new URL(contentRoot, here).href;
    const page = here.startsWith(buildRoot) ? here.slice(buildRoot.length).split("#", 1)[0] ?? "" : "";
    const rootOf = (environment) => site + environment.path;
    const visit = async (event, environment) => {
      if (page === "" || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey) {
        return;
      }
      event.preventDefault();
      location.assign(await pageIn(rootOf(environment), page));
    };
    const link = (environment, hasSummary) => {
      const { title, summary } = label(environment);
      const anchor = document.createElement("a");
      anchor.href = rootOf(environment) + page;
      anchor.addEventListener("click", (event) => {
        void visit(event, environment);
      });
      if (rootOf(environment) === buildRoot) {
        anchor.setAttribute("aria-current", "page");
        anchor.classList.add("sg-env-current");
      }
      const name = document.createElement("span");
      name.textContent = title;
      anchor.append(name);
      if (hasSummary) {
        const small = document.createElement("small");
        small.textContent = summary;
        anchor.append(small);
      }
      return anchor;
    };
    const fillMenus = (environments2) => {
      for (const trigger of document.querySelectorAll(".js-menu")) {
        if (trigger.textContent.trim() !== MENU_TITLE) {
          continue;
        }
        const id = trigger.getAttribute("aria-controls") ?? "";
        document.querySelector(`#${CSS.escape(id)}`)?.replaceChildren(
          ...environments2.map((environment) => {
            const item = document.createElement("li");
            item.append(link(environment, true));
            return item;
          })
        );
      }
    };
    const fillPage = (environments2) => {
      const container = document.querySelector("#sg-environments");
      if (container === null) {
        return;
      }
      const list = document.createElement("ul");
      for (const environment of environments2) {
        const item = document.createElement("li");
        item.append(link(environment, false), ` \u2014 ${label(environment).summary}`);
        if (environment.link !== void 0) {
          const source = document.createElement("a");
          source.href = environment.link;
          source.textContent = environment.kind === "pr" ? "pull request" : "source";
          item.append(" (", source, ")");
        }
        list.append(item);
      }
      container.replaceChildren(list);
    };
    const environments = parseEnvironments(await fetchIndex(site));
    if (environments.length === 0) {
      return;
    }
    fillMenus(environments);
    fillPage(environments);
  }
  if (typeof document !== "undefined" && document.currentScript instanceof HTMLScriptElement) {
    void start(document.currentScript.dataset["siteRoot"] ?? "");
  }
})();
