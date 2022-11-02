#!/bin/bash

Test=$TBASE/Tests/nginx
export FSIZES=( 4096 16384 65536 262144 1048576 4194304 16777216 67108864 )
export MAX_MEM=2147483648

for fsizel in ${FSIZES[@]};
do
	export FSIZE=$fsizel
	export DEST=$TBASE/nvme/mount/nginx.$FSIZE
	export DEST_WRK=$TBASE/nvme/mount/wrk.urls.$FSIZE
	echo $FSIZE $DEST $DEST_WRK
	$Test/generate_files.sh
done
