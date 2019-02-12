;; Calculation of Hofstadter's male and female sequences as a list of pairs

(define (hofstadter-male-female n)
 (letrec (
           (female
             (lambda (n)
              (if (= n 0)
               1
               (- n (male (female (- n 1)))))))
           (male (lambda (n)
                  (if (= n 0)
                   0
                   (- n (female (male (- n 1)))))))
         )
   (let loop ((i 0))
    (if (> i n)
     '()
     (cons (cons (female i)
            (male i))
      (loop (+ i 1)))))))

(hofstadter-male-female 8)


;; Building a list of squares from 0 to 9:
;; Note: loop is simply an arbitrary symbol used as a label. Any symbol will do.

(define (list-of-squares n)
 (let loop ((i n) (res '()))
  (if (< i 0)
   res
   (loop (- i 1) (cons (* i i) res)))))

(list-of-squares 9)