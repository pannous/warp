import subprocess, re, sys
FOREIGN = ()
def own_patch(path):
    diff = subprocess.run(["git","diff","-U1",path],capture_output=True,text=True).stdout
    header, *hunks = re.split(r'(?m)^(?=@@ )', diff)
    kept = [h for h in hunks if not any(word in h for word in FOREIGN)]
    return header + "".join(kept)
patch = "".join(own_patch(p) for p in sys.argv[1:])
subprocess.run(["git","apply","--cached","--recount","-"],input=patch,text=True,check=True)
