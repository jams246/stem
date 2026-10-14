function separated() {
  let amount = subtotal;
  one();
  two();
  three();
  four();
  five();
  six();
  seven();
  eight();
  nine();
  ten();
  amount -= discount;
  amount += delivery;
  return amount;
}

function grouped() {
  one();
  two();
  three();
  four();
  five();
  six();
  seven();
  eight();
  nine();
  ten();
  let amount = subtotal;
  amount -= discount;
  amount += delivery;
  return amount;
}
