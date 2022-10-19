# Reads csv files in `scripts/csvs`, and aggregates then into csv files
# that are used in `plot.py`.

import copy
import csv
import pathlib
import numpy as np
import scipy.stats
import os

COR_TX = "$\\mathbf{C_{Orig}}$ TX"
COR_RX = "$\\mathbf{C_{Orig}}$ RX"
SF_TX = "$\\mathbf{C_{SF}}$ TX"
SF_RX = "$\\mathbf{C_{SF}}$ RX"
MEL_TX = "$\\mathbf{M}$ TX"
MEL_RX = "$\\mathbf{M}$ RX"

d = {i: [] for i in range(1, 8+1)}
d = {'half': copy.deepcopy(d), 'full': copy.deepcopy(d)}
d = {'rx': copy.deepcopy(d), 'tx': copy.deepcopy(d)}
d = {1500: copy.deepcopy(d), 9000: copy.deepcopy(d)}
d = {'cor': copy.deepcopy(d), 'sf': copy.deepcopy(d), 'mel': copy.deepcopy(d)}

current_dir = pathlib.Path(f'{os.path.dirname(os.path.abspath(__file__))}')
pathlist = list(pathlib.Path(f'{current_dir}/../../csvs/iperf/').glob('*.csv'))

for n in pathlist:
    with open(n, 'r') as f:
        n = str(n)
        if 'corig' in n:
            target = 'cor'
        elif 'csf' in n:
            target = 'sf'
        elif 'm' in n:
            target = 'mel'
        else:
            continue
        csvreader = csv.reader(f)
        lines = []
        for r in csvreader:
            lines.append(r)
        if len(lines) != 22:
            continue

        # Excuse the code, it's 52 hrs until due
        for row_offset in [0, 11]:
            mtu = int(lines[row_offset][0][4:])
            for i in range(8):
                d[target][mtu]['tx']['half'][i+1].append(lines[row_offset+2+i][1])
                d[target][mtu]['rx']['half'][i+1].append(lines[row_offset+2+i][2])
                d[target][mtu]['tx']['full'][i+1].append(lines[row_offset+2+i][3])
                d[target][mtu]['rx']['full'][i+1].append(lines[row_offset+2+i][4])

print('Nums per dataset:')
cor = d['cor'][1500]['rx']['half'][1]
sf = d['sf'][1500]['rx']['half'][1]
mel = d['mel'][1500]['rx']['half'][1]
print(f'cor {len(cor)},\
        sf {len(sf)},\
        mel {len(mel)},')

for mtu in [1500, 9000]:
    l1 = []
    l2 = []
    sf_l = []
    mel_l = []
    for direc in ['tx', 'rx']:
        for duplex in ['half', 'full']:
            for i in range(1, 9):
                cor_ls = list(map(lambda x: float(x), d['cor'][mtu][direc][duplex][i]))
                sf_ls = list(map(lambda x: float(x), d['sf'][mtu][direc][duplex][i]))
                mel_ls = list(map(lambda x: float(x), d['mel'][mtu][direc][duplex][i]))
                cor_avg = np.average(cor_ls)
                sf_avg = np.average(sf_ls)
                mel_avg = np.average(mel_ls)
                l1.append(sf_avg/cor_avg - 1)
                l2.append(sf_avg/mel_avg - 1)
                sf_l.extend(sf_ls)
                mel_l.extend(mel_ls)


def pred(d, target, mtu, direction, duplex, i):
    org = d[target][mtu][direction][duplex][i]
    l_orig = len(org)
    x = list(map(lambda a: float(a), org))
    if len(x) < l_orig:
        print(target, mtu, direction, duplex , i)
        print(len(x), l_orig)
    avg = round(np.average(x), 2)
    std = round(np.std(x), 2)
    if std > 10:
        print(target, mtu, direction, duplex , i, ':', avg, std)
        print(org)

    return avg, std


for duplex in ['half', 'full']:
    for mtu in [1500, 9000]:
        with open(f'{current_dir}/{duplex}_{mtu}.csv', 'w') as f:
            f.write(f'parallel connections\ttype\tspeed\n')
            for target in ['sf', 'cor', 'mel']:
                for direction in ['tx', 'rx']:
                    if target == 'sf':
                        typ = SF_TX if direction == 'tx' else SF_RX
                    if target == 'cor':
                        typ = COR_TX if direction == 'tx' else COR_RX
                    if target == 'mel':
                        typ = MEL_TX if direction == 'tx' else MEL_RX
                    for i in range(1,9):
                        avg, std = pred(d, target, mtu, direction, duplex, i)
                        for b in d[target][mtu][direction][duplex][i]:
                            f.write(f'{i}\t{typ}\t{b}\n')
