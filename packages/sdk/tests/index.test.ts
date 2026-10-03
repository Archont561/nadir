import { expect, describe as suite, test } from "bun:test";
import { TRANSPORT_VERSION, describe, ping, version } from "../src/index";

suite("@nadir/sdk native transport", () => {
  test("round-trips through Rust", () => expect(ping("bun")).toBe("bun"));
  test("reads behavior from core", () => expect(describe()).toContain("nadir-core"));
  test("agrees on transport version", () => {
    expect(version().transportVersion).toBe(TRANSPORT_VERSION);
  });
});
