// The declarations in TypeScript and JavaScript files as the TypeScript compiler reads them, for
// utils/outline_check.py: per file, [kind, qualified name, first line, last line] in sundgren's
// kinds. Usage: node utils/ts_declarations.cjs PATH/TO/typescript < FILES, one path per line.
//
// Counted as sundgren counts them: functions, and bindings or class properties whose value is a
// function; prototypes (no body); classes, and bindings whose value is a class; interfaces, enums,
// type aliases and modules; methods of classes and interfaces; inside functions, only
// declarations.

const ts = require(process.argv[2]);
const fs = require("node:fs");

const files = fs.readFileSync(0, "utf8").split("\n").filter(Boolean);
const out = {};
for (const file of files) {
  const text = fs.readFileSync(file, "utf8");
  const x = /x$/.test(file);
  const scriptKind = /\.[cm]?tsx?$/.test(file)
    ? x
      ? ts.ScriptKind.TSX
      : ts.ScriptKind.TS
    : x
      ? ts.ScriptKind.JSX
      : ts.ScriptKind.JS;
  const source = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, scriptKind);
  const line = (position) => source.getLineAndCharacterOfPosition(position).line + 1;
  const nameOf = (node) => {
    if (ts.isConstructorDeclaration(node)) {
      return "constructor";
    }
    if (!node.name) {
      return "";
    }
    return ts.isStringLiteral(node.name) ? node.name.text : node.name.getText(source);
  };
  const isFunctionValue = (node) =>
    node.initializer !== undefined &&
    (ts.isArrowFunction(node.initializer) || ts.isFunctionExpression(node.initializer)) &&
    ts.isIdentifier(node.name);
  const kindOf = (node, inFunction) => {
    if (ts.isFunctionDeclaration(node)) {
      return node.body ? "function" : "prototype";
    }
    if (ts.isClassDeclaration(node)) return "class";
    if (ts.isInterfaceDeclaration(node)) return "interface";
    if (ts.isEnumDeclaration(node)) return "enum";
    if (ts.isTypeAliasDeclaration(node)) return "type";
    if (ts.isModuleDeclaration(node)) return "module";
    if (inFunction) return undefined;
    const method =
      ts.isMethodDeclaration(node) ||
      ts.isConstructorDeclaration(node) ||
      ts.isGetAccessor(node) ||
      ts.isSetAccessor(node);
    if (method && ts.isClassLike(node.parent)) {
      return node.body ? "function" : "prototype";
    }
    if (ts.isMethodSignature(node) && ts.isInterfaceDeclaration(node.parent)) return "prototype";
    if ((ts.isVariableDeclaration(node) || ts.isPropertyDeclaration(node)) && isFunctionValue(node)) {
      return "function";
    }
    if (ts.isVariableDeclaration(node) && node.initializer && ts.isClassExpression(node.initializer)) {
      return "class";
    }
    return undefined;
  };
  const declarations = [];
  const visit = (node, prefix, inFunction) => {
    const kind = kindOf(node, inFunction);
    let qualified = prefix;
    if (kind !== undefined) {
      const name = nameOf(node);
      qualified = prefix ? `${prefix}.${name}` : name;
      declarations.push([kind, qualified, line(node.getStart(source, false)), line(node.end)]);
    }
    const isFunction = ts.isFunctionLike(node) && node.body !== undefined;
    ts.forEachChild(node, (child) => {
      visit(child, qualified, inFunction || isFunction);
    });
  };
  visit(source, "", false);
  out[file] = declarations;
}
process.stdout.write(JSON.stringify(out));
