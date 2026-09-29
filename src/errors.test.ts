import { describe, expect, it } from "vitest";
import { errorText, isAuthError } from "./errors";

describe("errors", () => {
  it("detects auth errors", () => {
    expect(isAuthError({ kind: "NotLoggedIn" })).toBe(true);
    expect(isAuthError({ kind: "AuthExpired" })).toBe(true);
    expect(isAuthError({ kind: "Network", message: "x" })).toBe(false);
    expect(isAuthError(null)).toBe(false);
  });

  it("formats backend errors in Korean", () => {
    expect(errorText({ kind: "Network", message: "dns" })).toBe("네트워크 오류");
    expect(errorText({ kind: "Login", message: "access_denied" })).toBe("access_denied");
    expect(errorText({ kind: "Api", message: { status: 403, message: "forbidden" } })).toBe("forbidden");
    expect(errorText("plain")).toBe("plain");
    expect(errorText(new Error("boom"))).toBe("boom");
  });
});
