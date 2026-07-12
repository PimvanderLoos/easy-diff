/**
 * Controller for triggering the LLM analysis of a PR.
 *
 * Analysis is a manual, user-initiated action: the review screen renders the
 * diff immediately and leaves the category UI disabled until {@link runAnalysis}
 * is invoked (from the TitleBar "Analyze" button). Keeping the `invoke` call
 * here — rather than in `stores.ts` — keeps the store module free of Tauri IPC.
 *
 * Analysis state is reset via `resetReviewSession` in `stores.ts` when leaving a
 * PR, and inline on review-screen mount; there is no separate reset here.
 */

import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import type { AnalysisResult } from "./types.js";
import {
  analysisResult,
  analysisStatus,
  analysisErrorMsg,
  selectedPr,
} from "./stores.js";

/**
 * Runs the two-pass LLM analysis for the given PR in the background, updating
 * the analysis stores as it progresses.
 *
 * Sets {@link analysisStatus} to `"running"` immediately, then to `"done"` with
 * the populated {@link analysisResult}, or `"error"` with {@link analysisErrorMsg}
 * on failure. Never throws — failures surface through the stores (a dismissible
 * banner in the review screen) so the diff stays visible.
 *
 * Guards against two races: an in-flight run is never started twice (early
 * return while `"running"`), and a completed run only commits its result while
 * its PR is still the selected one — so navigating to another PR mid-run cannot
 * clobber the new PR's analysis stores with the old PR's late result.
 *
 * @param prNumber Platform PR number to analyse.
 */
export async function runAnalysis(prNumber: number): Promise<void> {
  // Drop concurrent triggers (e.g. retry button + TitleBar in the same tick).
  if (get(analysisStatus) === "running") return;

  analysisStatus.set("running");
  analysisErrorMsg.set(null);
  try {
    const result = await invoke<AnalysisResult>("run_analysis", { prNumber });
    // Only commit if this run's PR is still selected; otherwise a stale late
    // result would overwrite the now-current PR's stores.
    if (get(selectedPr)?.number !== prNumber) return;
    analysisResult.set(result);
    analysisStatus.set("done");
  } catch (e) {
    if (get(selectedPr)?.number !== prNumber) return;
    analysisErrorMsg.set(String(e));
    analysisStatus.set("error");
  }
}
