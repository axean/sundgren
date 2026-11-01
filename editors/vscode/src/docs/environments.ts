// The docs site's Development menu: every docs environment on the site (the live site, the
// preview of main and one preview per open pull request), read from environments.json at the
// site root, which tools/gh-pages.sh keeps current. Each links to this page in that environment,
// or to its front page if this page isn't there. Also fills #sg-environments on the Development
// page.

const MENU_TITLE = "Development";

/** One environment of environments.json (tools/environments.py). */
export interface Environment {
  readonly kind: "release" | "main" | "pr";
  /** Below the site root, ending in `/`. */
  readonly path: string;
  readonly title?: string;
  readonly sha?: string;
  readonly number?: number;
  readonly ref?: string;
  readonly link?: string;
}

const KINDS = new Set<string>(["release", "main", "pr"]);

type Fields = Readonly<Record<string, unknown>>;

function isObject(value: unknown): value is Fields {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function text(value: unknown): string | undefined {
  return typeof value === "string" && value !== "" ? value : undefined;
}

function parseEnvironment(value: unknown): Environment | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const { kind, path, number } = value;
  if (typeof path !== "string" || typeof kind !== "string" || !KINDS.has(kind)) {
    return undefined;
  }
  const title = text(value["title"]);
  const sha = text(value["sha"]);
  const ref = text(value["ref"]);
  const link = text(value["link"]);
  return {
    kind: kind as Environment["kind"],
    path,
    ...(title !== undefined && { title }),
    ...(sha !== undefined && { sha }),
    ...(typeof number === "number" && { number }),
    ...(ref !== undefined && { ref }),
    ...(link !== undefined && { link }),
  };
}

/** The environments of environments.json; the ones it can't read are left out. */
export function parseEnvironments(value: unknown): Environment[] {
  const list = isObject(value) ? value["environments"] : undefined;
  return Array.isArray(list)
    ? list.map((item) => parseEnvironment(item)).filter((item) => item !== undefined)
    : [];
}

function short(sha: string | undefined): string {
  return sha === undefined ? "" : sha.slice(0, 7);
}

/** An environment's name, and a line about it. */
export function label(environment: Environment): { title: string; summary: string } {
  if (environment.kind === "release") {
    return {
      title: "Live site",
      summary: `Release ${environment.title ?? ""}`.trim(),
    };
  }
  if (environment.kind === "main") {
    return { title: "main", summary: `Preview of main ${short(environment.sha)}`.trim() };
  }
  const number = environment.number === undefined ? "" : `#${String(environment.number)}`;
  const title = environment.title === undefined ? number : `${number} ${environment.title}`;
  const branch = environment.ref === undefined ? "" : `${environment.ref} `;
  return {
    title,
    summary: `Pull request preview: ${branch}${short(environment.sha)}`.trim(),
  };
}

/** This page in `environment` if it is there, else that environment's front page. */
async function pageIn(root: string, page: string): Promise<string> {
  try {
    const response = await fetch(root + page, { method: "HEAD" });
    return response.ok ? root + page : root;
  } catch {
    return root;
  }
}

/** environments.json, or undefined offline or before it is deployed. */
async function fetchIndex(site: string): Promise<unknown> {
  try {
    const response = await fetch(`${site}environments.json`, { cache: "no-cache" });
    return response.ok ? ((await response.json()) as unknown) : undefined;
  } catch {
    return undefined;
  }
}

async function start(site: string): Promise<void> {
  // This build's root URL, and this page's path below it.
  const here = location.href;
  const contentRoot = document.documentElement.dataset["content_root"] ?? "./";
  const buildRoot = new URL(contentRoot, here).href;
  const page = here.startsWith(buildRoot)
    ? (here.slice(buildRoot.length).split("#", 1)[0] ?? "")
    : "";
  const rootOf = (environment: Environment): string => site + environment.path;

  // Go to this page in `environment`, or to its front page if this page isn't there.
  const visit = async (event: MouseEvent, environment: Environment): Promise<void> => {
    if (page === "" || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey) {
      return;
    }
    event.preventDefault();
    location.assign(await pageIn(rootOf(environment), page));
  };

  const link = (environment: Environment, hasSummary: boolean): HTMLAnchorElement => {
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

  const fillMenus = (environments: readonly Environment[]): void => {
    for (const trigger of document.querySelectorAll(".js-menu")) {
      if (trigger.textContent.trim() !== MENU_TITLE) {
        continue;
      }
      const id = trigger.getAttribute("aria-controls") ?? "";
      document.querySelector(`#${CSS.escape(id)}`)?.replaceChildren(
        ...environments.map((environment) => {
          const item = document.createElement("li");
          item.append(link(environment, true));
          return item;
        }),
      );
    }
  };

  const fillPage = (environments: readonly Environment[]): void => {
    const container = document.querySelector("#sg-environments");
    if (container === null) {
      return;
    }
    const list = document.createElement("ul");
    for (const environment of environments) {
      const item = document.createElement("li");
      item.append(link(environment, false), ` — ${label(environment).summary}`);
      if (environment.link !== undefined) {
        const source = document.createElement("a");
        source.href = environment.link;
        source.textContent = environment.kind === "pr" ? "pull request" : "source";
        item.append(" (", source, ")");
      }
      list.append(item);
    }
    container.replaceChildren(list);
  };

  // Offline or not deployed yet: no environments, and the menu keeps its fixed links.
  const environments = parseEnvironments(await fetchIndex(site));
  if (environments.length === 0) {
    return;
  }

  fillMenus(environments);
  fillPage(environments);
}

// Only when loaded as the site's script (Sphinx adds it with data-site-root), not in tests.
if (typeof document !== "undefined" && document.currentScript instanceof HTMLScriptElement) {
  void start(document.currentScript.dataset["siteRoot"] ?? "");
}
