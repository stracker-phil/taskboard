import type { Slice } from "../store";

/** A question that interrupts before something is lost. */
export interface Confirmation {
  title: string;
  text: string;
  /** The destructive button's label, e.g. "Delete". */
  action: string;
  answer: (confirmed: boolean) => void;
}

export interface ConfirmSlice {
  confirmation: Confirmation | null;
  /** Shows the sheet; resolves true when the owner confirms, false on Cancel or Escape. */
  confirm: (question: Omit<Confirmation, "answer">) => Promise<boolean>;
}

export const confirmSlice: Slice<ConfirmSlice> = {
  create: (set, get) => ({
    confirmation: null,
    confirm: (question) =>
      new Promise<boolean>((resolve) => {
        get().confirmation?.answer(false);
        set({
          confirmation: {
            ...question,
            answer: (confirmed) => {
              set({ confirmation: null });
              resolve(confirmed);
            },
          },
        });
      }),
  }),
};
