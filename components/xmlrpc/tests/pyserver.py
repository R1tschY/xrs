from xmlrpc.server import SimpleXMLRPCRequestHandler
from xmlrpc.server import SimpleXMLRPCServer


class RequestHandler(SimpleXMLRPCRequestHandler):
    rpc_paths = ('/RPC2',)


def echo(*args):
    return args


def main():
    with SimpleXMLRPCServer(('localhost', 7777), requestHandler=RequestHandler) as server:
        server.register_introspection_functions()
        server.register_multicall_functions()
        server.register_function(echo, "xmlrpc.echo")

        print("Serving under http://localhost:7777/RPC2")
        server.serve_forever()


if __name__ == '__main__':
    main()
