;; two loops that call an async import (a WebAssembly.Suspending function) n times: one whose promise is already resolved (the pure cost of a suspension) and one that opens an OPFS sync handle
(module
  (import "env" "resolved" (func $resolved (result i32)))
  (import "env" "open" (func $open (param i32) (result i32)))
  (func (export "run_resolved") (param $n i32) (result i32)
    (local $i i32) (local $s i32)
    (block $done (loop $l
      (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
      (local.set $s (i32.add (local.get $s) (call $resolved)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $l)))
    (local.get $s))
  (func (export "run_open") (param $n i32) (result i32)
    (local $i i32) (local $s i32)
    (block $done (loop $l
      (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
      (local.set $s (i32.add (local.get $s) (call $open (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $l)))
    (local.get $s)))
