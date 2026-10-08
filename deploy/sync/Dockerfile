FROM ubuntu:24.04
COPY --chmod=755 target/release/sparkpad-sync-server /usr/local/bin/sparkpad-sync-server
COPY apps/sync-web/dist/ /app/web/
USER 1000:1000
ENV SPARKPAD_SERVER_BIND=0.0.0.0:7348 SPARKPAD_SERVER_DATA=/data SPARKPAD_SERVER_WEB=/app/web
EXPOSE 7348
ENTRYPOINT ["sparkpad-sync-server"]
