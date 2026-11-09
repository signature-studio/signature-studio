# void oracle_ms_call(const oracle_cpu *in, uint64_t fn, uint64_t sp)
# Loads all GPRs (except rsp) and XMM0-15 from *in, switches to the oracle stack at sp,
# calls fn (MS x64 ABI: the caller side has put the shadow space and stack arguments at
# sp..), and stores the registers at return into oracle_ret_cpu (same layout as oracle_cpu,
# with rsp at offset 32). Returns to the C caller with the host callee-saved registers
# restored. Never returns when fn faults: the child's signal handler ends the process.
    .text
    .globl oracle_ms_call
    .type oracle_ms_call,@function
oracle_ms_call:
    pushq %rbp
    pushq %rbx
    pushq %r12
    pushq %r13
    pushq %r14
    pushq %r15
    movq %rsp, host_rsp(%rip)
    movq %rsi, host_fn(%rip)
    movq %rdx, %rsp
    movq %rdi, %rax
    movups 128(%rax), %xmm0
    movups 144(%rax), %xmm1
    movups 160(%rax), %xmm2
    movups 176(%rax), %xmm3
    movups 192(%rax), %xmm4
    movups 208(%rax), %xmm5
    movups 224(%rax), %xmm6
    movups 240(%rax), %xmm7
    movups 256(%rax), %xmm8
    movups 272(%rax), %xmm9
    movups 288(%rax), %xmm10
    movups 304(%rax), %xmm11
    movups 320(%rax), %xmm12
    movups 336(%rax), %xmm13
    movups 352(%rax), %xmm14
    movups 368(%rax), %xmm15
    movq 8(%rax), %rcx
    movq 16(%rax), %rdx
    movq 24(%rax), %rbx
    movq 40(%rax), %rbp
    movq 48(%rax), %rsi
    movq 56(%rax), %rdi
    movq 64(%rax), %r8
    movq 72(%rax), %r9
    movq 80(%rax), %r10
    movq 88(%rax), %r11
    movq 96(%rax), %r12
    movq 104(%rax), %r13
    movq 112(%rax), %r14
    movq 120(%rax), %r15
    movq 0(%rax), %rax
    cld
    call *host_fn(%rip)
    movq %rax, oracle_ret_cpu+0(%rip)
    movq %rcx, oracle_ret_cpu+8(%rip)
    movq %rdx, oracle_ret_cpu+16(%rip)
    movq %rbx, oracle_ret_cpu+24(%rip)
    movq %rsp, oracle_ret_cpu+32(%rip)
    movq %rbp, oracle_ret_cpu+40(%rip)
    movq %rsi, oracle_ret_cpu+48(%rip)
    movq %rdi, oracle_ret_cpu+56(%rip)
    movq %r8, oracle_ret_cpu+64(%rip)
    movq %r9, oracle_ret_cpu+72(%rip)
    movq %r10, oracle_ret_cpu+80(%rip)
    movq %r11, oracle_ret_cpu+88(%rip)
    movq %r12, oracle_ret_cpu+96(%rip)
    movq %r13, oracle_ret_cpu+104(%rip)
    movq %r14, oracle_ret_cpu+112(%rip)
    movq %r15, oracle_ret_cpu+120(%rip)
    movups %xmm0, oracle_ret_cpu+128(%rip)
    movups %xmm1, oracle_ret_cpu+144(%rip)
    movups %xmm2, oracle_ret_cpu+160(%rip)
    movups %xmm3, oracle_ret_cpu+176(%rip)
    movups %xmm4, oracle_ret_cpu+192(%rip)
    movups %xmm5, oracle_ret_cpu+208(%rip)
    movups %xmm6, oracle_ret_cpu+224(%rip)
    movups %xmm7, oracle_ret_cpu+240(%rip)
    movups %xmm8, oracle_ret_cpu+256(%rip)
    movups %xmm9, oracle_ret_cpu+272(%rip)
    movups %xmm10, oracle_ret_cpu+288(%rip)
    movups %xmm11, oracle_ret_cpu+304(%rip)
    movups %xmm12, oracle_ret_cpu+320(%rip)
    movups %xmm13, oracle_ret_cpu+336(%rip)
    movups %xmm14, oracle_ret_cpu+352(%rip)
    movups %xmm15, oracle_ret_cpu+368(%rip)
    movq host_rsp(%rip), %rsp
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %rbx
    popq %rbp
    ret
    .size oracle_ms_call,.-oracle_ms_call

    .bss
    .align 8
host_rsp:
    .zero 8
host_fn:
    .zero 8
    .section .note.GNU-stack,"",@progbits
