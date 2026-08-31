%% x = x * (y * ((y * x) * (y * z)))  |-  x = (x * (y * (z * (w * u)))) * z
fof(hypothesis, axiom, ! [X,Y,Z] : (X = f(X,f(Y,f(f(Y,X),f(Y,Z)))))).
fof(goal, conjecture, ! [U,W,X,Y,Z] : (X = f(f(X,f(Y,f(Z,f(W,U)))),Z))).
