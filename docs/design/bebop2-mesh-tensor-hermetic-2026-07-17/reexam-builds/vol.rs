fn main(){
  let r=10.0f64;
  let tree = (1u64<<11)-1; // complete binary tree depth 10, nodes within 10 hops of root
  let eucl = std::f64::consts::PI*r*r;            // area of Euclidean disk radius 10
  let hyp  = 2.0*std::f64::consts::PI*(r.cosh()-1.0); // area of hyperbolic (curv -1) disk radius 10
  println!("binary-tree nodes within radius 10 = {}", tree);
  println!("Euclidean disk area  (R=10) = {:.0}", eucl);
  println!("Hyperbolic disk area (R=10, kappa=-1) = {:.0}", hyp);
  println!("hyperbolic/euclidean capacity ratio = {:.0}x", hyp/eucl);
}
