# Takes the csv files outputed by `aggregate.py`,
# and creates plot svg and png files.

import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
import pandas as pd
import pathlib
import os

COR = "$\\mathbf{C_{Orig}}$"
SF = "$\\mathbf{C_{SF}}$"
MEL = "$\\mathbf{M}$"

plots = [COR, SF, MEL]
point_color = {
    COR : '#dc5f57',
    SF : '#57dc5f',
    MEL : '#5f57dc',
}
hatches = ["//",  "-", "xx",]

filename = f'scale'
current_dir = pathlib.Path(f'{os.path.dirname(os.path.abspath(__file__))}')
sns.set()
sns.set_theme(style="ticks", font_scale=2, rc={'figure.figsize':(10,5)})

dt = pd.read_csv(f'{current_dir}/{filename}.csv', sep='\t')
dt['type'] = pd.Categorical(dt['type'], categories=plots, ordered=True)

fig = plt.figure()
ax = plt.axes()

# bar = sns.barplot(x="size", y="throughput", hue="type", data=dt, palette=point_color)
bar = sns.barplot(x="size", y="throughput", hue="type", data=dt, palette=point_color, ci='sd', errwidth=1.4, capsize=0.1)

# Loop over the bars
for bars, hatch in zip(ax.containers, hatches):
    # Set a different hatch for each group of bars
    for b in bars:
        b.set_hatch(hatch)
# create the legend again to show the new hatching
ax.legend()
# plt.show()
bar.set_xlabel('Client connections')
bar.set_ylabel('Throughput [Gbps]')

# remove every other xtick
xlabels = ax.get_xticklabels()
for i, l in enumerate(xlabels):
    if i%2 == 1:
        xlabels[i] = None
ax.set_xticklabels(xlabels)
xs = list(filter(lambda x: x%2==0, ax.get_xticks()))
ax.set_xticks(xs)

plt.legend(loc='lower right')
plt.tight_layout()
plt.savefig(f'{current_dir}/figure-scalability.svg')
