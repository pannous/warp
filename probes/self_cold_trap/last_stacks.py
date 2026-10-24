# the last distinct stacks of a WARP_PROFILE file (Firefox Profiler JSON), newest last: where a run was before it ended
import json, sys

def frame_names(thread, stack):
	stacks, frames, functions, names = thread['stackTable'], thread['frameTable'], thread['funcTable'], thread['stringArray']
	out = []
	while stack is not None:
		out.append(names[functions['name'][frames['func'][stacks['frame'][stack]]]])
		stack = stacks['prefix'][stack]
	return out

profile, count = json.load(open(sys.argv[1])), int(sys.argv[2]) if len(sys.argv) > 2 else 10
for thread in profile['threads']:
	seen = []
	for stack in thread['samples']['stack']:
		line = ' <- '.join(frame_names(thread, stack)[:14])
		if not seen or seen[-1] != line: seen.append(line)
	print('\n'.join(seen[-count:]))
