#!/bin/bash

[ -z "$FSIZE" ] && FSIZE=16384
[ -z "$MAX_MEM" ] && MAX_MEM=2147483648
[ -z "$DEST" ] && DEST=$TBASE/nvme/mount/nginx.$FSIZE
[ -z "$DEST_WRK" ] && DEST_WRK=$TBASE/nvme/mount/wrk.$FSIZE.urls

NUM_FILES=$[ $MAX_MEM / $FSIZE ]
BLOCK_COUNT=$[ $FSIZE / 4096 ]
 
Test=$TBASE/Tests/nginx
#$Test/config.sh
sudo mkdir -p $DEST

# generate sample
echo "while true ; do printf \"\xcc\xcc\xcc\xcc\xcc\xcc\xcc\xcc\"; done | sudo dd of=$DEST/nginx.$FSIZE.html bs=4k count=$BLOCK_COUNT iflag=fullblock ;"
while true ; do printf "\xcc\xcc\xcc\xcc\xcc\xcc\xcc\xcc"; done | sudo dd of=$DEST/nginx.$FSIZE.html bs=4k count=$BLOCK_COUNT iflag=fullblock;

# use sample
echo writing files... $NUM_FILES
for i in `seq 0 $NUM_FILES`;
do
	sudo cp $DEST/nginx.$FSIZE.html $DEST/file.$i.html
done

sudo sync

#ssh -t $dip1 mv $TBASE/wrk/urls.txt $TBASE/wrk/urls.txt.old
#echo writing wrk... $NUM_FILES
#sudo rm -f $DEST_WRK
#for i in `seq 0 $NUM_FILES`;
#do
	#ssh -t $dip1 "echo http://$dip1/file.$i.html >> $TBASE/wrk/urls.txt ;"
#	echo http://$dip1/file.$i.html | sudo tee -a $DEST_WRK > /dev/null;
#done 
