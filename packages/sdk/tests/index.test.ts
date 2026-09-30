import { describe as bunDescribe, expect, test } from "bun:test";

import { describe } from "../src/index";

bunDescribe("@nadir/sdk", () => {
  test("describe names the package", () => {
    expect(describe()).toContain("nadir/sdk");
  });
});
