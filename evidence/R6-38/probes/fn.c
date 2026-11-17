int add(int a, int b) { return a + b; }
extern int missing_fn(void);
int use(void) { return missing_fn() + 1; }
