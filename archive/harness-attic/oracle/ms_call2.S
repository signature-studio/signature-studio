# void oracle_ms_call(const oracle_cpu *in, uint64_t fn, uint64_t sp)   (oracle v2)
# As ms_call.S, plus: FNINIT and LDMXCSR 1F80H before the call (so the x87/MXCSR start state is
# the lift runtime's reset state), the input EFLAGS (offset 384, popfq; DF is cleared by the
# caller's mask, TF may be set to single-step the callee), and at return EFLAGS (pushfq, stored
# at 384), MXCSR (392) and the x87 environment (fnstenv, 400) into oracle_ret_cpu. TF is cleared
# again right after the return.
# Frame (a5f): the callee is entered by "jmp" with oracle_ret_target (OR_SENTINEL, 0xdead0000) pushed as
# its return address, the same value lift_drv2 pushes, so a function that saves [rsp] stores the same
# bytes on both sides. A small stub mapped at that address (native_drv2.c) jumps to oracle_ms_ret.
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
    fninit
    ldmxcsr mxcsr_def(%rip)
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
    pushq 384(%rax)
    popfq
    pushq oracle_ret_target(%rip)
    movq 0(%rax), %rax
    jmp *host_fn(%rip)
    .globl oracle_ms_ret
oracle_ms_ret:
    pushfq
    popq oracle_ret_cpu+384(%rip)
    pushq $2
    popfq
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
    stmxcsr oracle_ret_cpu+392(%rip)
    fnstenv oracle_ret_cpu+400(%rip)
    movq host_rsp(%rip), %rsp
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %rbx
    popq %rbp
    ret
    .size oracle_ms_call,.-oracle_ms_call

    .section .rodata
    .align 4
mxcsr_def:
    .long 0x1f80

    .bss
    .align 8
host_rsp:
    .zero 8
host_fn:
    .zero 8
    .section .note.GNU-stack,"",@progbits
