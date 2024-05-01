#!/bin/sh

has_certificate() {
  domain=$1;

  if [ -d /etc/letsencrypt/live/$domain ]; then
    return 0;
  else
    return 1;
  fi
}

has_every_certificate() {
  for domain in $CERTBOT_DOMAINS
  do
    if has_certificate $domain; then
      continue;
    else
      return 1;
    fi
  done

  return 0;
}

if has_every_certificate; then
  echo "SSL certificates already exist, skipping certbot"
  exit 0;
fi

sed -i -r 's/(listen .*443)/\1; #/g; s/(ssl_(certificate|certificate_key|trusted_certificate) )/#;#\1/g' /etc/nginx/conf.d/*;

service nginx start;

for domain in $CERTBOT_DOMAINS
do
  if has_certificate $domain; then
    continue;
  else
    certbot certonly -n --webroot -w /var/www/_letsencrypt -d ${domain} --email ${CERTBOT_EMAIL} --agree-tos;
  fi
done

service nginx stop;

sed -i -r -z 's/#?; ?#//g' /etc/nginx/conf.d/*;
