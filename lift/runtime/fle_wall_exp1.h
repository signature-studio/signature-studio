/* Walls that File -> Export -> Wave file reached (decision 0188, PROVISIONAL). Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9); the Win32 return value in R[0].
 * Each function is reached only through a row of harness/seeds/exp1-imports.txt (an earlier build has no row and stops at the import thunk as before). */
#ifndef FLE_WALL_EXP1_H
#define FLE_WALL_EXP1_H

/* kernel32!SetThreadExecutionState: EXECUTION_STATE f(EXECUTION_STATE esFlags ecx). */
void fle_imp_SetThreadExecutionState(void);

#endif
