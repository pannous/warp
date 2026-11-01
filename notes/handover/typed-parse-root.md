# Typed parse root diagnostic

Current main reports `expected object, received list of int` for the root-list outcome, a more precise diagnostic than its old expectation. The installed compiler reproduced it directly and the original outcome failed with that actual value. The outcome is upgraded to retain the element type. The historical `typed-parse` branch remains intact for its separately tracked follow-ups. Next run the focused outcome and merged-main smoke before landing.
