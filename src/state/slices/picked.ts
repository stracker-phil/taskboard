import type { Slice } from "../store";

export interface PickedSlice {
  /** The issues picked for a bulk change, per view (`narrowingKey`), in the order they show; never saved. */
  picked: Record<string, string[]>;
  setPicked: (key: string, ids: string[]) => void;
}

export const pickedSlice: Slice<PickedSlice> = {
  create: (set) => ({
    picked: {},
    setPicked: (key, ids) => set((state) => ({ picked: { ...state.picked, [key]: ids } })),
  }),
};
