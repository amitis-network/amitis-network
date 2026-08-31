import http.server, urllib.request, urllib.error

class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/':
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'OK')
        else:
            url = 'http://localhost:1317' + self.path
            try:
                r = urllib.request.urlopen(url)
                self.send_response(r.status)
                self.end_headers()
                self.wfile.write(r.read())
            except urllib.error.HTTPError as e:
                self.send_response(e.code)
                self.end_headers()
                self.wfile.write(e.read())
            except Exception as e:
                self.send_response(502)
                self.end_headers()
                self.wfile.write(str(e).encode())
    def do_POST(self):
        url = 'http://localhost:1317' + self.path
        length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(length)
        try:
            req = urllib.request.Request(url, data=body, method='POST')
            req.add_header('Content-Type', self.headers.get('Content-Type', 'application/json'))
            r = urllib.request.urlopen(req)
            self.send_response(r.status)
            self.end_headers()
            self.wfile.write(r.read())
        except urllib.error.HTTPError as e:
            self.send_response(e.code)
            self.end_headers()
            self.wfile.write(e.read())
        except Exception as e:
            self.send_response(502)
            self.end_headers()
            self.wfile.write(str(e).encode())
    def log_message(self, *args): pass

server = http.server.HTTPServer(('0.0.0.0', 1318), Handler)
server.allow_reuse_address = True
server.serve_forever()
