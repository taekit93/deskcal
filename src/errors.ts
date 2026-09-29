interface ErrorPayload { kind?: unknown; message?: unknown }

export function isAuthError(e: unknown): boolean {
  const kind = (e as ErrorPayload | null)?.kind;
  return kind === "NotLoggedIn" || kind === "AuthExpired";
}

export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  const p = e as ErrorPayload | null;
  if (p && typeof p.kind === "string") {
    if (p.kind === "NotLoggedIn") return "로그인이 필요합니다";
    if (p.kind === "AuthExpired") return "로그인이 만료되었습니다";
    if (p.kind === "Network") return "네트워크 오류";
    if (typeof p.message === "string") return p.message;
    if (p.message && typeof p.message === "object" && "message" in p.message) {
      return String((p.message as { message: unknown }).message);
    }
    return p.kind;
  }
  return String(e);
}
