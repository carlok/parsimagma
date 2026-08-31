%% x = x * (y * ((y * (x * z)) * y))  |-  x = (x * y) * ((x * x) * (y * z))
fof(hypothesis, axiom, ! [X,Y,Z] : (X = f(X,f(Y,f(f(Y,f(X,Z)),Y))))).
fof(goal, conjecture, ! [X,Y,Z] : (X = f(f(X,Y),f(f(X,X),f(Y,Z))))).
