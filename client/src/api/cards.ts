import type { Optional, Result } from "../types/effects";
import type { ApiError } from "./client";
import { apiGet, apiPost } from "./client";

/** Which of a Term's two texts a Card prompts with. `foreign` shows the
 *  foreign text and asks for the pivot text (recognition); `pivot` is
 *  the reverse (production). */
export type PromptSide = "foreign" | "pivot";

/** One direction of a Term, as served to the practice screen: the
 *  `prompt` to show, the `answer` to reveal, the parent Term's `notes`,
 *  and the Card's current Leitner `box` / `due_at`. */
export type PracticeCard = {
  id: string;
  term_id: string;
  prompt_side: PromptSide;
  prompt: string;
  answer: string;
  notes: Optional<string>;
  due_at: string;
  box: number;
};

/** The learner's self-assessment of one attempt. */
export type Rating = "pass" | "fail";

/** Cards due at or before `dueBefore` (ISO-8601), oldest-due first. The
 *  practice queue passes `limit`; the landing badge omits it to count
 *  everything currently due. */
export const listDueCards = (
  dueBefore: string,
  limit?: number,
): Promise<Result<PracticeCard[], ApiError>> => {
  const query =
    `due_before=${encodeURIComponent(dueBefore)}` +
    (limit === undefined ? "" : `&limit=${limit}`);
  return apiGet<PracticeCard[]>(`/cards?${query}`);
};

/** Grade one attempt at a Card. The server appends a review, reschedules
 *  the Card, and returns it updated. */
export const createReview = (
  cardId: string,
  rating: Rating,
): Promise<Result<PracticeCard, ApiError>> =>
  apiPost<PracticeCard>(
    `/cards/${encodeURIComponent(cardId)}/reviews`,
    { rating },
  );
