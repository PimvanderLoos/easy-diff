/**
 * Regex-based syntax tokeniser for Java and YAML source code.
 *
 * Returns an array of `{ t: string; c: string }` tokens where `t` is the raw
 * text fragment and `c` is a class hint used by `InlineDiff` to map to a CSS
 * syntax colour (`sx-keyword`, `sx-type`, `sx-string`, etc.).
 *
 * Special classes: `ws` (whitespace) and `ident` (plain identifier) are
 * rendered as-is (no colour override).
 *
 * @example
 * ```ts
 * import { highlight } from "$lib/syntax.js";
 * const tokens = highlight("MyFile.java", "public class Foo {");
 * ```
 */

export interface Token {
  /** Raw text fragment. */
  t: string;
  /** Class hint: "keyword" | "type" | "string" | "number" | "comment" |
   * "anno" | "method" | "ident" | "ws" | "punct" */
  c: string;
}

const KW =
  /^(public|private|protected|final|class|void|return|new|if|else|throw|throws|import|package|static|interface|enum|extends|implements|var|for|while|try|catch|null|true|false|this|super)\b/;
const TY = /^[A-Z][A-Za-z0-9_]*\b/;
const ANNO = /^@[A-Za-z][A-Za-z0-9_]*/;
const NUM = /^\d+([._]\d+)?/;
const ID = /^[a-zA-Z_$][a-zA-Z0-9_$]*/;
const WS = /^\s+/;
const STR = /^"([^"\\]|\\.)*"/;

/**
 * Tokenise a single line of Java (or Java-like) source code.
 */
export function highlightJava(text: string): Token[] {
  if (text == null) return [];

  // Whole-line comment shortcut.
  if (/^\s*\/\//.test(text) || /^\s*\/\*\*?/.test(text) || /^\s*\*/.test(text)) {
    return [{ t: text, c: "comment" }];
  }

  const out: Token[] = [];
  let i = 0;

  while (i < text.length) {
    const rest = text.slice(i);
    let m: RegExpMatchArray | null;

    if ((m = rest.match(WS))) {
      out.push({ t: m[0], c: "ws" });
      i += m[0].length;
      continue;
    }
    if ((m = rest.match(STR))) {
      out.push({ t: m[0], c: "string" });
      i += m[0].length;
      continue;
    }
    if ((m = rest.match(ANNO))) {
      out.push({ t: m[0], c: "anno" });
      i += m[0].length;
      continue;
    }
    if ((m = rest.match(KW))) {
      out.push({ t: m[0], c: "keyword" });
      i += m[0].length;
      continue;
    }
    if ((m = rest.match(TY))) {
      out.push({ t: m[0], c: "type" });
      i += m[0].length;
      continue;
    }
    if ((m = rest.match(NUM))) {
      out.push({ t: m[0], c: "number" });
      i += m[0].length;
      continue;
    }
    if ((m = rest.match(ID))) {
      const next = text[i + m[0].length];
      out.push({ t: m[0], c: next === "(" ? "method" : "ident" });
      i += m[0].length;
      continue;
    }
    out.push({ t: text[i], c: "punct" });
    i++;
  }

  return out;
}

/**
 * Tokenise a single line of YAML source code.
 */
export function highlightYaml(text: string): Token[] {
  if (!text) return [];
  const m = text.match(/^(\s*)([\w.-]+)(:)(.*)$/);
  if (!m) return [{ t: text, c: "ident" }];
  return [
    { t: m[1], c: "ws" },
    { t: m[2], c: "type" },
    { t: m[3], c: "punct" },
    { t: m[4], c: "string" },
  ];
}

/**
 * Dispatch to the appropriate highlighter based on file extension.
 *
 * Falls back to Java highlighting for unknown extensions.
 */
export function highlight(filePath: string | null | undefined, text: string): Token[] {
  if (filePath && filePath.endsWith(".yml")) return highlightYaml(text);
  if (filePath && filePath.endsWith(".yaml")) return highlightYaml(text);
  return highlightJava(text);
}
