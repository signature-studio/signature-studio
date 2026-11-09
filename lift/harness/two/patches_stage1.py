# TEMPORARY values for the measurement chain (NOT decisions, NOT runtime behaviour). Used with gdb_imports.py (GDBI_PATCHES).
# Stage 1: f_37d34b4 (CRT lowio init) passes an uninitialised stack buffer to GetStartupInfoW; fle_imp_GetStartupInfoW stops because
# [rcx] (cb) is not 0x68 (this stop rule is not in the text of decision 0018; it was only in a comment of fle_env.c and in env.rs, withdrawn by decision 0083). The Learn page of GetStartupInfoW does not say the caller
# must set cb. Measurement: write 0x68 into [rcx] before the call so that the runtime's own answer (all members zero, decision 0018)
# is given, and see what comes next.
SEEN = {}


def gsi(g):
    SEEN["cb_before"] = g.ld(g.reg(1), 4)
    print("   (GetStartupInfoW: [rcx]=%#x before the patch, rcx=%#x)" % (SEEN["cb_before"], g.reg(1)))
    g.st(g.reg(1), 4, 0x68)


PATCHES = {"fle_imp_GetStartupInfoW": gsi}
