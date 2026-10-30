TODO.md Usually To track open issues we're using the board 

https://github.com/users/pannous/projects/1/

If for some reason that's unreachable or GitHub is down, we can fall back to this manual file 

Here is a list of some random issues I encountered. You may remove them and create a proper card from them 


(type $Ratio (;4;) (struct (field $numerator anyref) (field $denominator anyref)))    anyref should be $BigInt $Node or $Number 


  (type (;242;) (func (result i64)))
  (type (;243;) (func (result i64)))
  Reuse identical types 

  (type (;76;) (func (param anyref anyref i64) (result anyref)))  Is there any reason why we use anyref not  $Node in ALL Function signatures? Bring to interviewer to discuss 

🌀2_+2
» Error("wasm trap: wasm `unreachable` instruction executed at /Users/me/dev/angles/warp/lib/compiler/compiler.warp:118:20")

warp beautify  No longer waits for the answers in the CLI assumes n