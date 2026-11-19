# The machine-wide slots that build-slot.sh (compiler builds) and job-slot.sh (wap tests, censuses) share out: one lock
# file a slot in a folder of the store, held while its command runs (source it)
LOCK_HELD_ELSEWHERE=75  # lockf's status when the slot is taken
# slot_tried <folder> <count> <command…>: the command run under the first free one of the folder's slots, its status;
# slot_held is no when every slot was taken and nothing ran. A command that itself exits 75 is told apart by the mark it
# leaves on starting
slot_tried() {
	local folder=$1 count=$2 slot status started
	shift 2
	mkdir -p "$folder"
	started=$folder/started.$$.$RANDOM
	for slot in $(seq "$count"); do
		lockf -s -t 0 "$folder/$slot" sh -c ': >"$0"; exec "$@"' "$started" "$@"
		status=$?
		if [ $status != $LOCK_HELD_ELSEWHERE ] || [ -e "$started" ]; then
			rm -f "$started"
			slot_held=yes
			return $status
		fi
	done
	slot_held=no
	return $LOCK_HELD_ELSEWHERE
}
