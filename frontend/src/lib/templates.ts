/// Substitutes `{token}` placeholders in a template body with the given values,
/// mirroring the backend's renderer. Unknown tokens are left intact so a typo
/// degrades gracefully rather than dropping text.
export function renderTemplate(body: string, vars: Record<string, string>): string {
  return body.replace(/\{(\w+)\}/g, (match, token) => vars[token] ?? match);
}
