; E2531 => E179 over a carrier of exactly 13
(set-logic ALL)
(declare-datatypes ((M 0)) (((e0) (e1) (e2) (e3) (e4) (e5) (e6) (e7) (e8) (e9) (e10) (e11) (e12))))
(declare-fun op (M M) M)
(assert (forall ((x0 M) (x1 M)) (= x0 (op (op x1 (op (op x1 x0) x0)) x1))))
(declare-const sk0 M)
(declare-const sk1 M)
(assert (not (= sk0 (op (op sk1 sk1) (op sk1 sk0)))))
(check-sat)
