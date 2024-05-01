FROM nginx:1.25.5

RUN apt update && \
    apt install -y certbot cron && \
    rm -r /etc/nginx/ && \
    mkdir -p /etc/nginx/conf.d && \
    mkdir -p /var/www/_letsencrypt

COPY scripts/initialize-certificates.sh /docker-entrypoint.d/40-initialize-certificates.sh
COPY scripts/start-cron.sh /docker-entrypoint.d/50-start-cron.sh
COPY scripts/reload-nginx.sh /etc/letsencrypt/renewal-hooks/post/reload-nginx.sh
COPY cron/ /etc/cron.d/

COPY config/ /etc/nginx/
