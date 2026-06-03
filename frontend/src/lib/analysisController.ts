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

import { invoke } from "@tauri-apps/api/core";
import type { AnalysisResult } from "./types.js";
import { analysisResult, analysisStatus, analysisErrorMsg } from "./stores.js";

/**
 * Runs the two-pass LLM analysis for the given PR in the background, updating
 * the analysis stores as it progresses.
 *
 * Sets {@link analysisStatus} to `"running"` immediately, then to `"done"` with
 * the populated {@link analysisResult}, or `"error"` with {@link analysisErrorMsg}
 * on failure. Never throws — failures surface through the stores (a dismissible
 * banner in the review screen) so the diff stays visible.
 *
 * @param prNumber Platform PR number to analyse.
 */
export async function runAnalysis(prNumber: number): Promise<void> {
  analysisStatus.set("running");
  analysisErrorMsg.set(null);
  try {
    const result = await invoke<AnalysisResult>("run_analysis", { prNumber });
    analysisResult.set(result);
    analysisStatus.set("done");
  } catch (e) {
    analysisErrorMsg.set(String(e));
    analysisStatus.set("error");
  }
}
