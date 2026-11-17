(module
  (import "env" "proc" (func $proc (param i32) (result i32)))
  (func (export "guest") (param $k i32) (param $n i32) (result i32)
    (local $i i32) (local $s i32)
    (block $done
      (loop $l
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $s (i32.add (local.get $s) (call $proc (local.get $k))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $l)))
    (local.get $s))
  (func (export "seq") (param $k i32) (param $n i32) (result i32)
    (local $i i32) (local $j i32) (local $s i32)
    (block $done
      (loop $l
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $j (i32.const 0))
        (block $d2
          (loop $m
            (br_if $d2 (i32.ge_u (local.get $j) (local.get $k)))
            (local.set $s (i32.add (local.get $s) (call $proc (local.get $j))))
            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $m)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $l)))
    (local.get $s)))
