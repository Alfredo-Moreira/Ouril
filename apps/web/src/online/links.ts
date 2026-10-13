/** The address someone opens to accept an invite link (`/join/:code`, ADR 0031). */
export function inviteLink(code: string): string {
  return `${window.location.origin}/join/${code}`;
}
