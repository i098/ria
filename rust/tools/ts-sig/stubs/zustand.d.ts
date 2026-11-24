// Ria original, no Ardia source.
// The part of the zustand types that `gen.mjs` needs: Ardia has no `node_modules` in the frozen checkout, so without this
// file `create` has type `any` and no store has a state type.
declare module "zustand" {
  type SetState<T> = (partial: Partial<T> | ((state: T) => Partial<T> | T), replace?: boolean) => void;
  export interface StoreApi<T> {
    getState(): T;
    setState: SetState<T>;
    subscribe(listener: (state: T, previous: T) => void): () => void;
  }
  export type StateCreator<T, Mis extends unknown[] = [], Mos extends unknown[] = [], U = T> = (
    set: SetState<T>,
    get: () => T,
    api: StoreApi<T>,
  ) => U;
  export type UseBoundStore<S> = S & { (): unknown };
  export function create<T>(): (init: StateCreator<T, [], [], T>) => UseBoundStore<StoreApi<T>>;
  export function create<T>(init: StateCreator<T, [], [], T>): UseBoundStore<StoreApi<T>>;
}
