#!/usr/bin/env bash

SCRIPT=$(readlink -f "$0")
SCRIPTPATH=$(dirname "$SCRIPT")
BASEDIR=$SCRIPTPATH/..
cd $BASEDIR

ARCHIVE=archive.zip
rm -rf $ARCHIVE
zip $ARCHIVE . -r --symlinks -x \
    build/\* \
    ci/\* \
    corundum/\* \
    .git/\* \
    .gitlab-ci.yml \
    macro/target/\* \
    target/\*
