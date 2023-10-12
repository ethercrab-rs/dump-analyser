# Copied from https://github.com/jiangcongtao/jupyter-rust/blob/main/Dockerfile

FROM rust:1.73-slim

# System packages
RUN apt-get update && apt-get install -y curl

# Install miniconda to /miniconda
RUN curl -LO http://repo.continuum.io/miniconda/Miniconda3-latest-Linux-x86_64.sh && \
    bash Miniconda3-latest-Linux-x86_64.sh -p /miniconda -b && \
    rm Miniconda3-latest-Linux-x86_64.sh

# configure & update conda
ENV PATH=/miniconda/bin:${PATH} \
    SHELL=/bin/bash
RUN conda init bash && \
    conda update -y conda && \
    conda install -c anaconda cmake -y && \
    conda install -y -c conda-forge nb_conda_kernels jupyterlab=4.0.7

# install evcxr_jupyter
RUN cargo install evcxr_jupyter && \
    evcxr_jupyter --install

EXPOSE 8888

CMD ["jupyter", "lab", "--ip=0.0.0.0", "--port=8888", "--notebook-dir=/opt/notebooks", "--allow-root", "--no-browser", "--NotebookApp.token=''", "--ServerApp.token=''"]
