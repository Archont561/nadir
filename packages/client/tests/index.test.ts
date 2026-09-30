import { describe as bunDescribe, expect, test } from "bun:test";

import { describe } from "../src/index";

bunDescribe("@nadir/client", () => {
  test("describe names the package and delegates to the sdk", () => {
    const description = describe();
    expect(description).toContain("nadir/client");
    expect(description).toContain("nadir/sdk");
  });
});
