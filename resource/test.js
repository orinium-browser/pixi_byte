println("=== Eval Test Start ===");

// Basic arithmetic
println(1 + 2 * 3);

// Variables
var x = "Variable";

println(x);

// Function definition + call
function add(a, b) {
  return a + b;
}

println(add(10, 20));

// Object
let obj = {
  value: 123,
  get: function () {
    return this.value;
  },
};

println(obj.get());

// this test
let a = {
  x: 10,
  show: function () {
    return this.x;
  },
};

let b = {
  x: 99,
  show: a.show,
};

println(a.show());
println(b.show());

println(a.show);
println(b.show);

const str = "hello";
println(str.length); // 5

function f(a, b, c) {}
println(f.length); // 3

println("=== Eval Test End ===");
