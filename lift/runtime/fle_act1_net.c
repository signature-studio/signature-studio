/* Decision 0213 (act1, PROVISIONAL), section 5. WinINet calls of FLEngine's HTTP unit (f_64d520, the account / license / FL Cloud / analytics requests) answered as "the server cannot be reached", with the failure
 * that the Microsoft pages document, instead of a stop. NOT A NETWORK MODEL. No request is sent; no response, no header, no cookie and no body byte ever exists; there is NO success answer for a transfer (a fake
 * answer to a license or authentication request is out of the question). The virtual machine has no connection (decision 0126, InternetGetConnectedState = FALSE).
 *
 * What the pages say (Learn, wininet.h; read 2026-11-04, docs/investigations/act1/sources/learn/):
 *  - InternetConnectW: "Returns a valid handle to the session if the connection is successful, or NULL otherwise"; "For FTP sites, InternetConnect actually establishes a connection with the server; for others, the
 *    actual connection is not established until the application requests a specific transaction." So for dwService HTTP (3) the call succeeds with no network and returns a handle (S-act1-connect-lazy follows the page).
 *  - HttpOpenRequestW: "Returns an HTTP request handle if successful, or NULL otherwise"; a NULL verb means GET; NULL lpszVersion / referrer / accept types are allowed. No connection is made here either (reading).
 *  - HttpAddRequestHeadersW: "Returns TRUE if successful"; dwHeadersLength -1 means zero-terminated; "Each header must be terminated by a CR/LF pair"; dwModifiers HTTP_ADDREQ_FLAG_ADD.
 *  - HttpSendRequestExW: "performs both the send and the receive"; FALSE with GetLastError on failure. This is where the missing connection shows: FALSE, ERROR_INTERNET_CANNOT_CONNECT (12029, "The attempt to connect to
 *    the server failed.", Learn "WinINet Error Codes") (S-act1-net-error: 12029 or ERROR_INTERNET_NAME_NOT_RESOLVED 12007, compile-time FLE_ACT1_NET_ERROR; the page does not say which of the two a machine without
 *    a connection gets, and decision 0132 answers InternetOpenUrlW with 12007).
 *  - InternetWriteFile, HttpEndRequestW, HttpQueryInfoA/W, InternetReadFile: "Returns TRUE if successful, or FALSE otherwise" + GetLastError. After a request that was never sent the handle "is not in the correct state":
 *    FALSE, ERROR_INTERNET_INCORRECT_HANDLE_STATE (12019) (S-act1-state-error, the page names no code per function).
 *  - InternetGetLastResponseInfoW: "Returns TRUE if error text was successfully written to the buffer"; with nothing stored it answers TRUE, error 0, an empty text (S-act1-lastresp-empty, UNSOURCED: a reading of
 *    what the function has to say when no server has ever answered).
 *  - InternetCloseHandle (fle_wininet.c) closes these handles through fle_act1_close_handle().
 * Handles: a region of their own (FLE_ACT1_HANDLE_BASE + 4 * (n + 1), never reused). Anything the guest has not been seen to do (another service than HTTP, a user name or password for InternetConnect, a handle
 * that is not ours, a request that is not a request) stops with the reason, as fle_wininet.c does.
 * LOG: one line per call on stderr: "fle: act1-net<TAB>#n<TAB>API<TAB>fields". Host, port, verb and object name are printed; of the object name's query only the parameter NAMES and the length of each value;
 * of the headers only the names and the total length; of a body only the byte count (S-act1-log-redaction: the engine puts e-mail, password and tokens in the query string and the headers). */
#include <stdint.h>
#include <stdio.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_imports.h"
#include "fle_wininet.h"
#include "fle_act1_net.h"

#define FLE_ACT1_HANDLE_BASE UINT64_C(0x7FFAB1000000)
#define FLE_ACT1_MAX 65536u        /* handle numbers are never reused below this; the table grows on demand */
#define FLE_ACT1_NET_ERROR 12029u   /* ERROR_INTERNET_CANNOT_CONNECT (the alternative 12007 was a compile-time choice; decision 0243 keeps one) */
#define FLE_ERROR_INTERNET_INCORRECT_HANDLE_STATE 12019u
#define INTERNET_SERVICE_HTTP 3u
#define HTTP_ADDREQ_FLAG_ADD 0x20000000u

typedef struct {
    int live;
    int kind;                 /* 1 connection, 2 request */
    int parent;               /* slot of the connection (kind 2) */
    char host[160];
    unsigned port;
    char verb[16];
    char obj[300];
    uint32_t flags;
    int sent;                 /* the send was tried (and failed) */
    uint32_t hdr_total;       /* characters of headers added */
    uint64_t body_bytes;      /* bytes the guest tried to write */
} act1_ent;

static act1_ent *ent;
static unsigned ent_n, ent_cap, seq_n;

/* room for one more entry (the table is allocated on first use and doubles); stops only at FLE_ACT1_MAX handles */
static void ent_room(const char *api)
{
    if (ent_n == ent_cap) {
        unsigned nc = ent_cap ? ent_cap * 2u : 64u;
        act1_ent *p;

        if (ent_n >= FLE_ACT1_MAX) fle_stop(api, "more connection and request handles than the runtime holds (65536, none reused)");
        p = realloc(ent, nc * sizeof *ent);
        if (p == NULL) fle_stop(api, "out of memory");
        ent = p;
        ent_cap = nc;
    }
}

static void wide_to_ascii_x(char *out, size_t cap, uint64_t p, uint32_t n, int keep_crlf)
{
    size_t i = 0;

    if (p == 0) { snprintf(out, cap, "(NULL)"); return; }
    for (; i + 1 < cap; i++) {
        uint16_t c;
        if (n != 0xFFFFFFFFu && i >= n) break;
        c = (uint16_t)fle_ld(p + 2 * i, 2);
        if (c == 0 && n == 0xFFFFFFFFu) break;
        out[i] = (c >= 0x20 && c < 0x7f) || (keep_crlf && (c == '\r' || c == '\n')) ? (char)c : '?';
    }
    out[i] = 0;
}

static void wide_to_ascii(char *out, size_t cap, uint64_t p, uint32_t n)
{
    wide_to_ascii_x(out, cap, p, n, 0);
}

static int slot_of(uint64_t h, int kind)
{
    uint64_t k;

    if (h <= FLE_ACT1_HANDLE_BASE || (h - FLE_ACT1_HANDLE_BASE) % 4u) return -1;
    k = (h - FLE_ACT1_HANDLE_BASE) / 4u - 1u;
    if (k >= ent_n || !ent[k].live || ent[k].kind != kind) return -1;
    return (int)k;
}

/* the object name with every query value replaced by its length: "/a/b?username=<12>&password=<9>" */
static void redact_obj(char *out, size_t cap, const char *obj)
{
    const char *q = obj + strcspn(obj, "?&");   /* the engine also writes "...downloadnewsrss&flsdata=...&id=..." (no '?') */
    size_t n;
    int first = 1;

    if (!*q) { snprintf(out, cap, "%s", obj); return; }
    n = (size_t)(q - obj) < cap - 1 ? (size_t)(q - obj) : cap - 1;
    memcpy(out, obj, n);
    out[n] = 0;
    q++;
    while (*q && n + 60 < cap) {
        const char *amp = strchr(q, '&');
        size_t plen = amp ? (size_t)(amp - q) : strlen(q);
        const char *eq = memchr(q, '=', plen);
        size_t klen = eq ? (size_t)(eq - q) : plen;
        size_t vlen = eq ? plen - klen - 1 : 0;
        if (klen > 40) klen = 40;
        n += (size_t)snprintf(out + n, cap - n, "%c%.*s=<%zu>", first ? '?' : '&', (int)klen, q, vlen);
        first = 0;
        if (!amp) break;
        q = amp + 1;
    }
}

/* the header NAMES of "Name: value\r\nName: value\r\n" joined by ',' */
static void header_names(char *out, size_t cap, const char *h)
{
    size_t n = 0;

    out[0] = 0;
    while (*h && n + 2 < cap) {
        const char *eol = strstr(h, "\r\n");
        size_t linelen = eol ? (size_t)(eol - h) : strlen(h);
        const char *colon = memchr(h, ':', linelen);
        size_t nl = colon ? (size_t)(colon - h) : linelen;
        if (nl > 40) nl = 40;
        n += (size_t)snprintf(out + n, cap - n, "%s%.*s", n ? "," : "", (int)nl, h);
        if (!eol) break;
        h = eol + 2;
    }
}

static void note(const char *api, const char *fmt, ...) __attribute__((format(printf, 2, 3)));
static void note(const char *api, const char *fmt, ...)
{
    va_list ap;

    fprintf(stderr, "fle: act1-net\t#%u\t%s\t", ++seq_n, api);
    va_start(ap, fmt);
    vfprintf(stderr, fmt, ap);
    va_end(ap);
    fputc('\n', stderr);
}

/* wininet!InternetConnectW: HINTERNET f(HINTERNET rcx, LPCWSTR lpszServerName rdx, INTERNET_PORT nServerPort r8w, LPCWSTR lpszUserName r9, LPCWSTR lpszPassword [5], DWORD dwService [6], DWORD dwFlags [7], DWORD_PTR dwContext [8]). */
void fle_imp_InternetConnectW(void)
{
    static const char api[] = "InternetConnectW";
    uint32_t service = (uint32_t)fle_arg(6), flags = (uint32_t)fle_arg(7);
    act1_ent *e;

    if (!fle_inet_session_live(R[1])) fle_stop(api, "hInternet is not a live session of this runtime: the page names no error code (decision 0213)");
    if (R[2] == 0) fle_stop(api, "lpszServerName is NULL: the page gives no behaviour (decision 0213)");
    if (service != INTERNET_SERVICE_HTTP) fle_stop(api, "dwService is not INTERNET_SERVICE_HTTP: FTP opens a real connection and needs a model of its own (decision 0213)");
    if (R[9] != 0 || fle_arg(5) != 0) fle_stop(api, "a user name or password is passed: the guest has not been seen to do it and the page gives no HTTP behaviour (decision 0213)");
    if (flags != 0) fle_stop(api, "dwFlags is not 0: the guest has not been seen to pass flags (decision 0213)");
    ent_room(api);
    e = &ent[ent_n];
    memset(e, 0, sizeof *e);
    e->live = 1;
    e->kind = 1;
    wide_to_ascii(e->host, sizeof e->host, R[2], 0xFFFFFFFFu);
    e->port = (unsigned)(R[8] & 0xFFFFu);
    R[0] = FLE_ACT1_HANDLE_BASE + 4u * (ent_n + 1u);
    note(api, "host=%s\tport=%u\tservice=HTTP\tflags=0x0\t-> handle %#llx (no connection is made: Learn)", e->host, e->port, (unsigned long long)R[0]);
    ent_n++;
}

/* wininet!HttpOpenRequestW: HINTERNET f(HINTERNET rcx, LPCWSTR lpszVerb rdx, LPCWSTR lpszObjectName r8, LPCWSTR lpszVersion r9, LPCWSTR lpszReferrer [5], LPCWSTR *lplpszAcceptTypes [6], DWORD dwFlags [7], DWORD_PTR dwContext [8]). */
void fle_imp_HttpOpenRequestW(void)
{
    static const char api[] = "HttpOpenRequestW";
    int c = slot_of(R[1], 1);
    uint32_t flags = (uint32_t)fle_arg(7);
    act1_ent *e;
    char red[340];

    if (c < 0) fle_stop(api, "hConnect is not a connection handle of this runtime: the page names no error code (decision 0213)");
    if (R[8] == 0) fle_stop(api, "lpszObjectName is NULL: the page gives no behaviour (decision 0213)");
    ent_room(api);
    e = &ent[ent_n];
    memset(e, 0, sizeof *e);
    e->live = 1;
    e->kind = 2;
    e->parent = c;
    e->flags = flags;
    if (R[2] == 0) snprintf(e->verb, sizeof e->verb, "GET");
    else wide_to_ascii(e->verb, sizeof e->verb, R[2], 0xFFFFFFFFu);
    wide_to_ascii(e->obj, sizeof e->obj, R[8], 0xFFFFFFFFu);
    redact_obj(red, sizeof red, e->obj);
    R[0] = FLE_ACT1_HANDLE_BASE + 4u * (ent_n + 1u);
    note(api, "host=%s\tport=%u\tverb=%s\tobject=%s\tflags=%#x\tversion=%s\treferrer=%s\t-> handle %#llx", ent[c].host, ent[c].port, e->verb, red, (unsigned)flags,
         R[9] ? "given" : "NULL", fle_arg(5) ? "given" : "NULL", (unsigned long long)R[0]);
    ent_n++;
}

/* wininet!HttpAddRequestHeadersW: BOOL f(HINTERNET rcx, LPCWSTR lpszHeaders rdx, DWORD dwHeadersLength r8d, DWORD dwModifiers r9d). */
void fle_imp_HttpAddRequestHeadersW(void)
{
    static const char api[] = "HttpAddRequestHeadersW";
    int r = slot_of(R[1], 2);
    uint32_t len = (uint32_t)R[8], mod = (uint32_t)R[9];
    char buf[1200], names[300];

    if (r < 0) fle_stop(api, "hRequest is not a request handle of this runtime: the page names no error code (decision 0213)");
    if (R[2] == 0) fle_stop(api, "lpszHeaders is NULL: the page gives no behaviour (decision 0213)");
    if (mod != HTTP_ADDREQ_FLAG_ADD) fle_stop(api, "dwModifiers is not HTTP_ADDREQ_FLAG_ADD: the guest has not been seen to pass another modifier (decision 0213)");
    wide_to_ascii_x(buf, sizeof buf, R[2], len, 1);
    header_names(names, sizeof names, buf);
    ent[r].hdr_total += len == 0xFFFFFFFFu ? (uint32_t)strlen(buf) : len;
    R[0] = 1;
    note(api, "host=%s\theader-names=%s\tchars=%u\tmodifiers=ADD\t-> TRUE (the headers are only noted)", ent[ent[r].parent].host, names, (unsigned)(len == 0xFFFFFFFFu ? strlen(buf) : len));
}

/* wininet!HttpSendRequestExW: BOOL f(HINTERNET rcx, LPINTERNET_BUFFERSW lpBuffersIn rdx, LPINTERNET_BUFFERSW lpBuffersOut r8, DWORD dwFlags r9d, DWORD_PTR dwContext [5]). */
void fle_imp_HttpSendRequestExW(void)
{
    static const char api[] = "HttpSendRequestExW";
    int r = slot_of(R[1], 2);
    char red[340];

    if (r < 0) fle_stop(api, "hRequest is not a request handle of this runtime: the page names no error code (decision 0213)");
    if (R[2] != 0 || R[8] != 0 || (uint32_t)R[9] != 0) fle_stop(api, "lpBuffersIn, lpBuffersOut or dwFlags is not 0: the guest has not been seen to pass them (decision 0213)");
    if (ent[r].sent) fle_stop(api, "the request was already sent once: the page gives no behaviour for a second send (decision 0213)");
    ent[r].sent = 1;
    redact_obj(red, sizeof red, ent[r].obj);
    fle_set_last_error(FLE_ACT1_NET_ERROR);
    R[0] = 0;
    note(api, "host=%s\tport=%u\tverb=%s\tobject=%s\theader-chars=%u\t-> FALSE, last error %u (NOT SENT: no connection, S-act1-net-error)", ent[ent[r].parent].host, ent[ent[r].parent].port, ent[r].verb, red,
         (unsigned)ent[r].hdr_total, (unsigned)FLE_ACT1_NET_ERROR);
}

static void bad_state(const char *api, int r, const char *what)
{
    fle_set_last_error(FLE_ERROR_INTERNET_INCORRECT_HANDLE_STATE);
    R[0] = 0;
    note(api, "host=%s\t%s\t-> FALSE, last error %u (the request was never sent; S-act1-state-error)", ent[ent[r].parent].host, what, (unsigned)FLE_ERROR_INTERNET_INCORRECT_HANDLE_STATE);
}

/* wininet!InternetWriteFile: BOOL f(HINTERNET rcx, LPCVOID lpBuffer rdx, DWORD dwNumberOfBytesToWrite r8d, LPDWORD lpdwNumberOfBytesWritten r9). */
void fle_imp_InternetWriteFile(void)
{
    static const char api[] = "InternetWriteFile";
    int r = slot_of(R[1], 2);
    char what[60];

    if (r < 0) fle_stop(api, "hFile is not a request handle of this runtime: the page names no error code (decision 0213)");
    if (R[9] != 0) fle_st(R[9], 4, 0);
    ent[r].body_bytes += (uint32_t)R[8];
    snprintf(what, sizeof what, "bytes=%u", (unsigned)R[8]);
    bad_state(api, r, what);
}

/* wininet!HttpEndRequestW: BOOL f(HINTERNET rcx, LPINTERNET_BUFFERSW lpBuffersOut rdx, DWORD dwFlags r8d, DWORD_PTR dwContext r9). */
void fle_imp_HttpEndRequestW(void)
{
    static const char api[] = "HttpEndRequestW";
    int r = slot_of(R[1], 2);
    char what[80];

    if (r < 0) fle_stop(api, "hRequest is not a request handle of this runtime: the page names no error code (decision 0213)");
    snprintf(what, sizeof what, "body-bytes-written=%llu", (unsigned long long)ent[r].body_bytes);
    bad_state(api, r, what);
}

static void query_info(const char *api)
{
    int r = slot_of(R[1], 2);
    char what[80];

    if (r < 0) fle_stop(api, "hRequest is not a request handle of this runtime: the page names no error code (decision 0213)");
    snprintf(what, sizeof what, "info-level=%#x", (unsigned)R[2]);
    bad_state(api, r, what);
}

/* wininet!HttpQueryInfoW / HttpQueryInfoA: BOOL f(HINTERNET rcx, DWORD dwInfoLevel edx, LPVOID lpBuffer r8, LPDWORD lpdwBufferLength r9, LPDWORD lpdwIndex [5]). */
void fle_imp_HttpQueryInfoW(void) { query_info("HttpQueryInfoW"); }
void fle_imp_HttpQueryInfoA(void) { query_info("HttpQueryInfoA"); }

/* wininet!InternetReadFile: BOOL f(HINTERNET rcx, LPVOID lpBuffer rdx, DWORD dwNumberOfBytesToRead r8d, LPDWORD lpdwNumberOfBytesRead r9). */
void fle_imp_InternetReadFile(void)
{
    static const char api[] = "InternetReadFile";
    int r = slot_of(R[1], 2);
    char what[60];

    if (r < 0) fle_stop(api, "hFile is not a request handle of this runtime: the page names no error code (decision 0213)");
    if (R[9] != 0) fle_st(R[9], 4, 0);
    snprintf(what, sizeof what, "want=%u", (unsigned)R[8]);
    bad_state(api, r, what);
}

/* wininet!InternetGetLastResponseInfoW: BOOL f(LPDWORD lpdwError rcx, LPWSTR lpszBuffer rdx, LPDWORD lpdwBufferLength r8). */
void fle_imp_InternetGetLastResponseInfoW(void)
{
    static const char api[] = "InternetGetLastResponseInfoW";

    if (R[1] == 0 || R[8] == 0) fle_stop(api, "lpdwError or lpdwBufferLength is NULL: the page gives no behaviour (decision 0213)");
    fle_st(R[1], 4, 0);
    if (R[2] != 0 && (uint32_t)fle_ld(R[8], 4) >= 1) fle_st(R[2], 2, 0);
    fle_st(R[8], 4, 0);
    R[0] = 1;
    note(api, "-> TRUE, error 0, empty text (no server has answered; S-act1-lastresp-empty, unsourced)");
}

/* Called by fle_wininet.c's InternetOpenUrlW (a weak reference; decision 0132 answers it with ERROR_INTERNET_NAME_NOT_RESOLVED): one record with the URL, its query values replaced by their lengths. */
void fle_act1_note_open_url(uint64_t url, uint64_t headers, uint32_t flags)
{
    char u[400], red[440], *p, *q;

    wide_to_ascii(u, sizeof u, url, 0xFFFFFFFFu);
    p = strstr(u, "://");
    q = p ? strchr(p + 3, '/') : NULL;
    if (q) {
        char host[160];
        size_t hl = (size_t)(q - (p + 3)) < sizeof host - 1 ? (size_t)(q - (p + 3)) : sizeof host - 1;
        memcpy(host, p + 3, hl);
        host[hl] = 0;
        redact_obj(red, sizeof red, q);
        note("InternetOpenUrlW", "scheme=%.*s\thost=%s\tobject=%s\theaders=%s\tflags=%#x\t-> NULL, last error 12007 (decision 0132)", (int)(p - u), u, host, red, headers ? "given" : "NULL", (unsigned)flags);
    } else {
        note("InternetOpenUrlW", "url-without-path (%zu chars)\theaders=%s\tflags=%#x\t-> NULL, last error 12007 (decision 0132)", strlen(u), headers ? "given" : "NULL", (unsigned)flags);
    }
}

int fle_act1_close_handle(uint64_t h)
{
    int k;

    if (h <= FLE_ACT1_HANDLE_BASE || (h - FLE_ACT1_HANDLE_BASE) % 4u) return 0;
    k = (int)((h - FLE_ACT1_HANDLE_BASE) / 4u - 1u);
    if ((unsigned)k >= ent_n || !ent[k].live) return 0;
    ent[k].live = 0;
    note("InternetCloseHandle", "kind=%s\thost=%s\t-> TRUE", ent[k].kind == 1 ? "connection" : "request", ent[k].kind == 1 ? ent[k].host : ent[ent[k].parent].host);
    return 1;
}
