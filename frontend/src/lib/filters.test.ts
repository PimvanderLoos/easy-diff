/**
 * Unit tests for the diff filter logic in `filters.ts`.
 *
 * Covers tag-resolution precedence (per-line overrides vs hunk classification)
 * and the change-type / attention-tag gating in `linePasses`.
 */
import { describe, it, expect } from "vitest";
import { lineTags, linePasses } from "./filters.js";
import type { DiffLineData, Classification, FilterState } from "./types.js";

const ctxLine: DiffLineData = { type: "ctx", text: "unchanged" };
const addLine: DiffLineData = { type: "add", text: "added", tags: [] };

const classification: Classification = {
  changeType: "feature",
  attentionTags: ["security"],
};

describe("lineTags", () => {
  it("prefers explicit per-line tags over the hunk classification", () => {
    const line = { ...addLine, tags: ["nitpick"] };
    expect(lineTags(line, classification)).toEqual(["nitpick"]);
  });

  it("falls back to the hunk classification tags", () => {
    expect(lineTags(addLine, classification)).toEqual(["security"]);
  });
});

describe("linePasses", () => {
  const filter: FilterState = { changeType: "all", attentionTags: ["security"] };

  it("always passes context lines", () => {
    expect(linePasses(ctxLine, classification, filter)).toBe(true);
  });

  it("passes add lines that carry a selected tag", () => {
    expect(linePasses(addLine, classification, filter)).toBe(true);
  });

  it("rejects add lines missing every selected tag", () => {
    const other: Classification = {
      changeType: "feature",
      attentionTags: ["docs"],
    };
    expect(linePasses(addLine, other, filter)).toBe(false);
  });
});
