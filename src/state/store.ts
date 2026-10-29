import { create, type StoreApi } from "zustand";
import { confirmSlice, type ConfirmSlice } from "./slices/confirm";
import { menuSlice, type MenuSlice } from "./slices/menu";
import { narrowingSlice, type NarrowingSlice } from "./slices/narrowing";
import { paletteSlice, type PaletteSlice } from "./slices/palette";
import { issueViewSlice, type IssueViewSlice } from "./slices/issueView";
import { placesSlice, type PlacesSlice } from "./slices/places";
import { uiSlice, type UiSlice } from "./slices/ui";

export { fail } from "./requests";
export type { Toast, ToastAction } from "./slices/ui";
export type { Confirmation } from "./slices/confirm";

/** Every slice's members; modules add theirs with `declare module "../state/store" { interface Store { … } }`. */
export interface Store extends UiSlice, ConfirmSlice, NarrowingSlice, PlacesSlice, IssueViewSlice, PaletteSlice, MenuSlice {}

type SetStore = StoreApi<Store>["setState"];
type GetStore = StoreApi<Store>["getState"];

export interface Slice<T> {
  /** Initial state and actions; also re-run by `resetStore`. */
  create(set: SetStore, get: GetStore): T;
}

// Empty until the slices below register; registerSlice fills it.
export const useStore = create<Store>()(() => ({}) as Store);

const slices: Slice<Partial<Store>>[] = [];

/** Adds a slice's state and actions to the live store; works after creation (modules register late). */
export function registerSlice<T extends Partial<Store>>(slice: Slice<T>) {
  slices.push(slice);
  useStore.setState(slice.create(useStore.setState, useStore.getState));
}

/** Restores every registered slice's initial state, then applies `patch`; for tests. */
export function resetStore(patch: Partial<Store> = {}) {
  const initial = slices.reduce<Partial<Store>>((state, slice) => ({ ...state, ...slice.create(useStore.setState, useStore.getState) }), {});
  useStore.setState({ ...initial, ...patch });
}

registerSlice(uiSlice);
registerSlice(confirmSlice);
registerSlice(narrowingSlice);
registerSlice(placesSlice);
registerSlice(issueViewSlice);
registerSlice(paletteSlice);
registerSlice(menuSlice);
