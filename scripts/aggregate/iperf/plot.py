# Takes the csv files outputed by `aggregate.py`,
# and creates plot svg and png files.

import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
import pandas as pd
import pathlib
import os
import itertools

def flip(items, ncol):
    return itertools.chain(*[items[i::ncol] for i in range(ncol)])

# COR_TX = "$\\mathbf{C_{Orig}}$ TX"
# COR_RX = "$\\mathbf{C_{Orig}}$ RX"
# SF_TX = "$\\mathbf{C_{SF}}$ TX"
# SF_RX = "$\\mathbf{C_{SF}}$ RX"
# MEL_TX = "$\\mathbf{M}$ TX"
# MEL_RX = "$\\mathbf{M}$ RX"
COR = "$\\mathbf{C_{Orig}}$"
SF = "$\\mathbf{C_{SF}}$"
MEL = "$\\mathbf{M}$"

# plots = [COR_TX, SF_TX, MEL_TX, COR_RX,  SF_RX,  MEL_RX]
# plots = [COR_TX, COR_RX, SF_TX, SF_RX, MEL_TX, MEL_RX]
plots = [COR, SF, MEL]

# point_shape = {
#     COR_TX : 's',
#     COR_RX : 'D',
#     SF_TX : '^',
#     SF_RX : 'v',
#     MEL_TX : 'o',
#     MEL_RX : '*',
# }
# point_shape = ['s', 'D', '^', 'v', 'o', '*']
point_shape = ['s', '^', 'o', 'D', 'v', '*']

# point_color = {
#     COR_TX : '#dc5f57',
#     COR_RX : '#95261f',
#     SF_TX : '#57dc5f',
#     SF_RX : '#1f9526',
#     MEL_TX : '#5f57dc',
#     MEL_RX : '#261f95'
# }
point_color = {
    COR : '#dc5f57',
    SF : '#57dc5f',
    MEL : '#5f57dc',
}
hatches = ['/', '-', 'x', "//",  "--", "xx",]

# def change_width(ax, new_value) :
#     for i, patch in enumerate(ax.patches):
#         if (i // 8) % 6 < 3:
#             offset = -new_value
#         else:
#             offset = new_value
#         patch.set_x(patch.get_x() + offset)
#     for i, eb in enumerate(ax.lines):
#         xy = eb.get_xydata()
#         if i < 72:
#             offset = -new_value
#         else:
#             offset = new_value
#         eb.set_xdata([xy[0][0] + offset, xy[1][0] + offset])

current_dir = pathlib.Path(f'{os.path.dirname(os.path.abspath(__file__))}')
sns.set()
sns.set_theme(style="ticks", font_scale=2, rc={'figure.figsize':(10,4.2)})

for mtu in [1500, 9000]:
    for duplex in ['half', 'full']:
        for direction in ['rx', 'tx']:
            filename = f'{duplex}_{mtu}_{direction}'
            dt = pd.read_csv(current_dir / (filename+'.csv'), sep='\t')
            dt['type'] = pd.Categorical(dt['type'], categories=plots, ordered=True)

            fig = plt.figure()
            ax = plt.axes()

            bar = sns.pointplot(x="parallel connections", y="speed", hue="type", ci='sd', data=dt, palette=point_color, markers=point_shape, capsize=0.15, )

            # Set marker size
            for c in ax.collections:
                c.set_sizes([200]*8)
            # Set line color
            for l in ax.get_lines():
                xy = l.get_xydata()
                # xy values different - not error bar or cap
                if xy[0][0] != xy[1][0] and xy[0][1] != xy[1][1]:
                    l.set_color('gray')
                    l.set_linestyle('--')
                    l.set_linewidth(2.5)

            # bar = sns.barplot(x="parallel connections", y="speed", hue="type", data=dt, palette=point_color, ci='sd', errwidth=1.4, capsize=0.07)

            # # Loop over the bars
            # for bars, hatch in zip(ax.containers, hatches):
            #     # Set a different hatch for each group of bars
            #     for b in bars:
            #         b.set_hatch(hatch)
            # change_width(ax, .05)

            # create the legend again to show the new hatching
            ax.legend()
            bar.set_xlabel('iperf processes')
            bar.set_ylabel('Throughput [Gbps]')

            # plt.legend(loc='lower right', ncol=2, columnspacing=0.3, handletextpad=-0.3)
            handles, labels = ax.get_legend_handles_labels()
            plt.legend(flip(handles, 3), flip(labels, 3), loc='lower right', ncol=3, columnspacing=0.3, handletextpad=.6)
            ax.set(ylim=(0,100))
            plt.tight_layout()
            output_name = f'figure-perf-{mtu}-{duplex}-{direction}'
            # plt.savefig(f'{current_dir}/{output_name}.png')
            plt.savefig(f'{current_dir}/{output_name}.svg')
