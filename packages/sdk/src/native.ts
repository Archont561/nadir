/** Loader and type for the N-API transport adapter. */
import { createRequire } from "node:module";

export interface NadirAddon {
  invoke(request: string): string;
}

const addon = createRequire(import.meta.url)(
  "../nadir-node-native.linux-x64-gnu.node",
) as NadirAddon;

export default addon;
