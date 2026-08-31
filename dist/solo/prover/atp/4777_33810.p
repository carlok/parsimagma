%% x = x * (y * (x * (z * (z * z))))  |-  x = ((x * y) * (z * (z * w))) * z
fof(hypothesis, axiom, ! [X,Y,Z] : (X = f(X,f(Y,f(X,f(Z,f(Z,Z))))))).
fof(goal, conjecture, ! [W,X,Y,Z] : (X = f(f(f(X,Y),f(Z,f(Z,W))),Z))).
