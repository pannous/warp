import re,sys,os
# usage: defify.py [--write] <folders>
write='--write' in sys.argv
roots=[a for a in sys.argv[1:] if a!='--write']
head=re.compile(r'^(\s*)([A-Za-z_][A-Za-z0-9_]*[?!]?)\(')
definition_tail=re.compile(r'^\s*(:\s*[^=:{}"\']+?)?\s*:=')
def closing(s,i):
    depth=0;quote=None
    while i<len(s):
        c=s[i]
        if quote:
            if c=='\\': i+=1
            elif c==quote: quote=None
        elif c in '"\'': quote=c
        elif c=='(': depth+=1
        elif c==')':
            depth-=1
            if depth==0: return i
        i+=1
    return -1
total=0
for root in roots:
  for dp,_,fs in os.walk(root):
    for f in sorted(fs):
        if not f.endswith('.warp'): continue
        p=os.path.join(dp,f); lines=open(p,encoding='utf8').readlines(); changed=False
        for n,l in enumerate(lines):
            m=head.match(l)
            if not m or m.group(2)[0].isupper(): continue
            e=closing(l,m.end()-1)
            if e<0 or not definition_tail.match(l[e+1:]): continue
            total+=1; changed=True
            if not write or m.group(1): print(f'{p}:{n+1}: {l.rstrip()[:120]}')
            lines[n]=m.group(1)+'def '+l[len(m.group(1)):]
        if write and changed: open(p,'w',encoding='utf8').writelines(lines)
print(total,file=sys.stderr)
