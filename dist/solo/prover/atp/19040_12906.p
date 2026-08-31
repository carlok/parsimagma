%% x = (y * x) * ((z * z) * (x * z))  |-  x = y * ((x * (z * (z * y))) * w)
fof(hypothesis, axiom, ! [X,Y,Z] : (X = f(f(Y,X),f(f(Z,Z),f(X,Z))))).
fof(goal, conjecture, ! [W,X,Y,Z] : (X = f(Y,f(f(X,f(Z,f(Z,Y))),W)))).
