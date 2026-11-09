/* Decision 0132 (PROVISIONAL, wall 24, S-errmsg-wininet-learn-text): message texts of the message tables of virtual modules, for FormatMessageW(FORMAT_MESSAGE_FROM_HMODULE). Hand-written from Microsoft Learn, "Appendix: WinINet Error
 * Codes" (docs/investigations/seh2/learn/WinINet-errors.txt), exactly as the page holds the description. A READING: that wininet.dll's real message table holds this text is NOT measured (a real table may word it differently).
 * Only the code that the guest has asked about. */
static const struct { const char *module; uint32_t code; const char *name; const char *text; } fle_errmsg_module[] = {
    { "wininet.dll", 12007u, "ERROR_INTERNET_NAME_NOT_RESOLVED", "The server name could not be resolved." },
};
