# 1. Build both images (reads server/Containerfile and client/Containerfile)
# podman build -t flashcards-server ./server
# podman build -t flashcards-client ./client

# 2. Create the network and the persistent volume once
# podman network create flashcards-net
# podman volume create flashcards_sqlite_data

# 3. Run the server. Name it "server" — the client's nginx config
#    (client/nginx.conf) proxies /api/ to http://server:8080, resolving
#    that hostname via Podman's network DNS.
podman run -d --name server --network flashcards-net \
  -v flashcards_sqlite_data:/data -p 8080:8080 localhost/flashcards-server

# 4. Run the client
podman run -d --name client --network flashcards-net \
  -p 3000:80 localhost/flashcards-client
 
