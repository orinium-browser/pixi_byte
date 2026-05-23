this.println("=== Eval Test Start ===");

// Basic arithmetic
this.println(1 + 2 * 3);

// Variables
var x = "Variable";

this.println(x);

// Function definition + call
function add(a, b) {
  return a + b;
}

this.println(add(10, 20));

// Object
let obj = {
  value: 123,
  get: function () {
    return this.value;
  },
};

this.println(obj.get());

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

this.println(a.show());
this.println(b.show());

this.println("=== Eval Test End ===");
