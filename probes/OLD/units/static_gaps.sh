#!/bin/sh
# Forms units::answer computed at compile time that static_units must take (card cleanup-units-answer)
WARP="${WARP:-warp}"
for code in '(2 km)²' '(2 km)^2' '(3 m)³' 'temperature = 40°C; temperature' '98.6°F' '300K' '40°C as °F' '212°F as °C' '0°C as K' '0K in °F' '40°C > 100°F' '0°C == 32°F' '20°C + 5°C' '20°C + 50°F' '6 m² as cm²' '6 m² in cm²' '2 m³ as cm³' '1 mi as m' '1 mile in feet' '1 ft + 1 inch' '2 m * 3 kg' 'kg*m/s²' 'N = kg*m/s²; 2*N' '1 km/h + 1 m/s' '(23/18)m/s' '10 km / 3 h' '(10/3)km/h' '1m+1kg'; do
	printf '%-34s → ' "$code"
	WARP_NO_WINDOW=1 "$WARP" --no-hints "$code" 2>&1 | tail -1 | tr '\n' ' ' | cut -c1-150
	echo
done
