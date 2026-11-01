#!/bin/zsh
# Links the default registry's packages into a tree's packages/ folder, so programs and tests reach them by path,
# `use "packages/colors/colors.warp"` (decision package-use-by-path: the compiler looks nothing up by name).
# The registry github.com/pannous/warp-packages is cloned once to ~/.cache/warp/packages/.registry and pulled when
# older than an hour. A packages/<name> that is a real folder stays; a link is pointed at the registry's folder.
# usage: scripts/link-packages.sh [<tree>]   (default: this checkout)
registry_url=https://github.com/pannous/warp-packages.git
registry=${WARP_PACKAGES_CACHE:-$HOME/.cache/warp/packages}/.registry
tree=${1:-${0:A:h:h}}
max_age_minutes=60

if [[ ! -d $registry/.git ]]; then
  git clone -q $registry_url $registry || { print -u2 "link-packages: cannot clone $registry_url"; exit 1 }
elif [[ -n $(find $registry/.git/FETCH_HEAD -mmin +$max_age_minutes 2>/dev/null) || ! -e $registry/.git/FETCH_HEAD ]]; then
  git -C $registry pull -q --ff-only || print -u2 "link-packages: pull failed, using the clone as it is"
fi
mkdir -p $tree/packages
for folder in $registry/*(/); do
  name=${folder:t}
  target=$tree/packages/$name
  if [[ -d $target && ! -L $target ]]; then continue; fi
  ln -sfn $folder $target
done
print "link-packages: $tree/packages linked to $registry"
